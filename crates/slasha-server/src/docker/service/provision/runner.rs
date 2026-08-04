use bollard::{Docker, plugin::ProgressDetail, query_parameters::CreateImageOptions};
use futures_util::StreamExt;
use slasha_db::{
    app::App,
    logs::LogPrefix,
    repos::service::ServiceRepo,
    service::{Service, ServiceStatus},
};

use super::instance;
use crate::{
    docker::{
        DockerError, DockerResult,
        naming::{service_container_name, service_volume_name},
        service::{resolve_service_env, spec::ServiceKindDockerExt},
        utils::{self, stream_container_logs},
        workflow::runner::WorkflowContext,
    },
    logs::LogWriter,
    state::AppState,
};

pub struct ServiceProvisionRunner<'a> {
    pub state: &'a AppState,
    pub app: &'a App,
    pub service: &'a Service,
    pub docker_client: &'a Docker,
    pub wf: &'a WorkflowContext<'a>,
    pub log: &'a LogWriter,
}

impl<'a> ServiceProvisionRunner<'a> {
    /// Executes the database service provisioning workflow sequentially.
    ///
    /// # Arguments
    ///
    /// * `is_redeploy` - Whether this workflow is redeploying an existing service.
    pub async fn execute(&self, is_redeploy: bool) -> DockerResult<()> {
        let db_pool = &self.state.storage.db_pool;

        ServiceRepo::update_status(db_pool, &self.service.id, ServiceStatus::Provisioning).await?;

        let image_name = self.service.kind.docker_image(&self.service.version);

        self.log
            .stdout(format!("Pulling Docker image {}", image_name));

        let mut stream = self.docker_client.create_image(
            Some(CreateImageOptions {
                from_image: Some(image_name.clone()),
                ..Default::default()
            }),
            None,
            None,
        );

        while let Some(result) = stream.next().await {
            let info = result?;
            if let Some(status) = info.status {
                let msg = match info.progress_detail {
                    Some(ProgressDetail {
                        current: Some(current),
                        total: Some(total),
                    }) => format!("{}: {}/{}", status, current, total),
                    _ => status,
                };
                self.log.stdout(msg);
            }
        }

        if let Ok(inspect) = self.docker_client.inspect_image(&image_name).await
            && let Some(repo_digests) = inspect.repo_digests
            && let Some(digest) = repo_digests.into_iter().next()
        {
            ServiceRepo::update_image_digest(db_pool, &self.service.id, &digest).await?;
        }

        let volume_name = service_volume_name(&self.service.id);
        let container_name = service_container_name(&self.service.id);

        self.wf
            .step(
                format!("Creating persistent volume {}", volume_name),
                async {
                    utils::create_volume(self.docker_client, &volume_name, None).await?;
                    Ok::<(), DockerError>(())
                },
                {
                    let docker_client = self.docker_client.clone();
                    let volume_name = volume_name.clone();

                    async move {
                        if !is_redeploy
                            && let Err(e) =
                                utils::remove_volume(&docker_client, &volume_name)
                                    .await
                        {
                            tracing::warn!(volume = %volume_name, error = ?e, "Failed to remove volume during rollback");
                        }
                    }
                },
            )
            .await?;

        let env_vars = ServiceRepo::get_env_vars(db_pool, &self.service.id).await?;
        let resolved_vars = resolve_service_env(env_vars, self.service)?;

        self.wf
            .step(
                format!("Creating service container {}", container_name),
                instance::create_service_container(
                    self.docker_client,
                    self.service,
                    self.app,
                    &resolved_vars,
                ),
                {
                    let docker_client = self.docker_client.clone();
                    let container_name = container_name.clone();
                    async move {
                        if let Err(e) =
                            utils::remove_container(&docker_client, &container_name)
                                .await
                        {
                            tracing::warn!(container = %container_name, error = ?e, "Failed to remove container during rollback");
                        }
                    }
                },
            )
            .await?;

        self.log
            .stdout(format!("Starting service {}", self.service.name));

        utils::start_container(self.docker_client, &container_name).await?;

        stream_container_logs(
            self.docker_client.clone(),
            self.log.clone().prefix(LogPrefix::Service),
            container_name.clone(),
        );

        instance::wait_for_service_health(
            self.docker_client,
            &container_name,
            &self.service.name,
            180,
            self.log,
        )
        .await?;

        ServiceRepo::update_status(db_pool, &self.service.id, ServiceStatus::Running).await?;

        Ok(())
    }
}
