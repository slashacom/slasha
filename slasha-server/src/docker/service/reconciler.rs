use std::time::Duration;

use bollard::Docker;
use slasha_db::{
    DbPool,
    repos::service::ServiceRepo,
    service::{Service, ServiceStatus},
};
use tokio::time::sleep;

use crate::docker::naming::service_container_name;

const RECONCILE_INTERVAL: Duration = Duration::from_secs(30);

pub fn spawn_service_reconciler(docker_client: Docker, db_pool: DbPool) {
    tokio::spawn(async move {
        loop {
            sleep(RECONCILE_INTERVAL).await;

            if let Err(e) = reconcile_once(&docker_client, &db_pool).await {
                tracing::error!("Service reconciler pass failed: {:?}", e);
            }
        }
    });
}

async fn reconcile_once(docker_client: &Docker, db_pool: &DbPool) -> anyhow::Result<()> {
    let services = ServiceRepo::list_non_terminal(db_pool).await?;

    for (svc, _) in services {
        if svc.status != ServiceStatus::Running {
            continue;
        }

        reconcile_running_service(docker_client, db_pool, &svc).await;
    }

    Ok(())
}

async fn reconcile_running_service(docker_client: &Docker, db_pool: &DbPool, svc: &Service) {
    let name = service_container_name(&svc.id);

    let next_status = match docker_client.inspect_container(&name, None).await {
        Ok(info) => {
            let running = info
                .state
                .as_ref()
                .and_then(|s| s.running)
                .unwrap_or(false);

            if running {
                return;
            }

            ServiceStatus::Stopped
        }
        Err(_) => ServiceStatus::Failed,
    };

    if let Err(e) = ServiceRepo::update_status(db_pool, &svc.id, next_status).await {
        tracing::error!(
            "Failed to reconcile service {} to {:?}: {:?}",
            svc.id,
            next_status,
            e
        );
        return;
    }

    tracing::info!(
        "Reconciled service {} ({}): Running -> {:?}",
        svc.name,
        svc.id,
        next_status
    );
}
