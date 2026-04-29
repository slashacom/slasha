use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::extract::FromRef;
use bollard::Docker;
use slasha_db::DbPool;
use tokio::sync::Notify;

use crate::{
    docker::{logs::LogManager, port_pool::PortPool},
    proxy::CaddyClient,
    utils,
};

#[derive(Clone)]
pub struct Clients {
    pub docker: Docker,
    pub caddy: CaddyClient,
}

impl Clients {
    pub fn new(docker: Docker) -> Self {
        Self {
            docker,
            caddy: CaddyClient::default(),
        }
    }
}

#[derive(Clone)]
pub struct Storage {
    pub db_pool: DbPool,
    pub repos_dir: PathBuf,
}

impl Storage {
    pub fn new(db_path: &std::path::Path, repos_dir: PathBuf) -> anyhow::Result<Self> {
        let db_str = db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid DB path"))?;
        let db_pool = slasha_db::create_pool(db_str)?;
        Ok(Self { db_pool, repos_dir })
    }
}

#[derive(Clone)]
pub struct Runtime {
    pub port_pool: Arc<PortPool>,
    pub log_manager: Arc<LogManager>,
    pub proxy_reconcile: Arc<Notify>,
}

impl Runtime {
    pub async fn new(
        port_start: u16,
        port_end: u16,
        docker_client: &Docker,
        logs_dir: &Path,
        proxy_reconcile: Arc<Notify>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            port_pool: Arc::new(PortPool::new(port_start, port_end, docker_client).await?),
            log_manager: Arc::new(LogManager::new(utils::ensure_dir(logs_dir))),
            proxy_reconcile,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Env {
    Development,
    Production,
}

impl Env {
    pub fn from_str_or_default(s: &str) -> Self {
        match s {
            "production" => Env::Production,
            _ => Env::Development,
        }
    }

    pub fn is_production(self) -> bool {
        matches!(self, Env::Production)
    }
}

#[derive(Clone)]
pub struct Config {
    pub env: Env,
    pub jwt_secret: String,
    pub platform_domain: Option<String>,
    pub logs_dir: PathBuf,
}

impl Config {
    pub fn new(
        env: Env,
        jwt_secret: String,
        platform_domain: Option<String>,
        logs_dir: PathBuf,
    ) -> Self {
        Self {
            env,
            jwt_secret,
            platform_domain,
            logs_dir,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub clients: Clients,
    pub storage: Storage,
    pub runtime: Runtime,
    pub config: Config,
}

impl AppState {
    pub fn new(config: Config, clients: Clients, storage: Storage, runtime: Runtime) -> Self {
        Self {
            clients,
            storage,
            runtime,
            config,
        }
    }
}

impl FromRef<AppState> for Clients {
    fn from_ref(state: &AppState) -> Self {
        state.clients.clone()
    }
}

impl FromRef<AppState> for Storage {
    fn from_ref(state: &AppState) -> Self {
        state.storage.clone()
    }
}

impl FromRef<AppState> for Runtime {
    fn from_ref(state: &AppState) -> Self {
        state.runtime.clone()
    }
}

impl FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}
