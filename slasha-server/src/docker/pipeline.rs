use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bollard::Docker;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use models::app::App;
use models::deployment::{Deployment, DeploymentStatus};

use super::broadcaster::DeploymentBroadcaster;
use super::build::{
    BuildStrategy, detect_build_strategy, phase_build_docker, phase_build_railpack,
};
use super::port_pool::PortPool;
use super::run::{phase_run, update_deployment_status};
use super::utils::detect_container_port;
use crate::error::{DeploymentError, Result};

const RAILPACK_CONTAINER_PORT: u16 = 8080;

pub async fn run_deployment(
    docker: Arc<Docker>,
    pool: Arc<PortPool>,
    broadcaster: Arc<DeploymentBroadcaster>,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    app: App,
    deployment: Deployment,
) -> Result<()> {
    if let Err(e) =
        run_deployment_inner(&docker, &pool, &broadcaster, &db_pool, &app, &deployment).await
    {
        tracing::error!("Deployment {} failed: {:?}", deployment.id, e);

        broadcaster
            .send(&deployment.id, format!("Deployment failed: {}", e))
            .await?;

        broadcaster.remove(&deployment.id);

        if let Ok(mut conn) = db_pool.get() {
            let _ = update_deployment_status(&mut conn, &deployment.id, DeploymentStatus::Failed);
        }
    }

    Ok(())
}

async fn run_deployment_inner(
    docker: &Arc<Docker>,
    pool: &Arc<PortPool>,
    broadcaster: &Arc<DeploymentBroadcaster>,
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    app: &App,
    deployment: &Deployment,
) -> Result<()> {
    let deployment_id = &deployment.id;
    let repo_path = Path::new(&app.repo_path);

    let strategy = detect_build_strategy(repo_path, &deployment.commit_sha).await?;

    {
        let mut conn = db_pool.get().map_err(DeploymentError::PoolError)?;
        update_deployment_status(&mut conn, deployment_id, DeploymentStatus::Building)?;
    }

    match strategy {
        BuildStrategy::Dockerfile { content } => {
            let container_port = detect_container_port(&content);

            broadcaster
                .send(
                    deployment_id,
                    format!(
                        "Building image slasha/{}:{} (Dockerfile)",
                        app.slug, deployment.commit_sha
                    ),
                )
                .await?;

            phase_build_docker(docker, broadcaster, app, deployment).await?;

            phase_run(
                docker,
                db_pool,
                broadcaster,
                pool,
                app,
                deployment,
                container_port,
                None,
            )
            .await?;
        }

        BuildStrategy::Railpack => {
            broadcaster
                .send(
                    deployment_id,
                    format!(
                        "Building image slasha/{}:{} (Railpack)",
                        app.slug, deployment.commit_sha
                    ),
                )
                .await?;

            phase_build_railpack(docker, broadcaster, app, deployment).await?;

            let mut env = HashMap::new();
            env.insert("PORT".to_string(), RAILPACK_CONTAINER_PORT.to_string());

            phase_run(
                docker,
                db_pool,
                broadcaster,
                pool,
                app,
                deployment,
                RAILPACK_CONTAINER_PORT,
                Some(env),
            )
            .await?;
        }
    }

    Ok(())
}
