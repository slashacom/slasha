use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::extract::FromRef;
use bollard::Docker;
use slasha_db::DbPool;
use tokio::sync::Notify;

use crate::{connections::GithubClient, docker::logs::LogManager, proxy::CaddyClient, utils};

#[derive(Clone)]
pub struct Clients {
    pub docker: Docker,
    pub caddy: CaddyClient,
    pub github: Option<GithubClient>,
}

impl Clients {
    pub fn new(docker: Docker, github: Option<GithubClient>) -> Self {
        Self {
            docker,
            caddy: CaddyClient::default(),
            github,
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
    pub log_manager: Arc<LogManager>,
    pub proxy_sync_trigger: Arc<Notify>,
    pub scaling_locks: Arc<dashmap::DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    pub connection_sync_locks: Arc<dashmap::DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl Runtime {
    pub async fn new(logs_dir: &Path, proxy_sync_trigger: Arc<Notify>) -> anyhow::Result<Self> {
        Ok(Self {
            log_manager: Arc::new(LogManager::new(utils::ensure_dir(logs_dir))),
            proxy_sync_trigger,
            scaling_locks: Arc::new(dashmap::DashMap::new()),
            connection_sync_locks: Arc::new(dashmap::DashMap::new()),
        })
    }

    pub fn get_scaling_lock(&self, deployment_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.scaling_locks
            .entry(deployment_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    pub fn get_connection_sync_lock(&self, app_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.connection_sync_locks
            .entry(app_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
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
    pub platform_domain: String,
    pub logs_dir: PathBuf,
    pub port: u16,
}

impl Config {
    pub fn new(
        env: Env,
        jwt_secret: String,
        platform_domain: String,
        logs_dir: PathBuf,
        port: u16,
    ) -> Self {
        Self {
            env,
            jwt_secret,
            platform_domain,
            logs_dir,
            port,
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

impl FromRef<AppState> for Docker {
    fn from_ref(state: &AppState) -> Self {
        state.clients.docker.clone()
    }
}

impl FromRef<AppState> for CaddyClient {
    fn from_ref(state: &AppState) -> Self {
        state.clients.caddy.clone()
    }
}

impl FromRef<AppState> for Storage {
    fn from_ref(state: &AppState) -> Self {
        state.storage.clone()
    }
}

impl FromRef<AppState> for DbPool {
    fn from_ref(state: &AppState) -> Self {
        state.storage.db_pool.clone()
    }
}

impl FromRef<AppState> for Runtime {
    fn from_ref(state: &AppState) -> Self {
        state.runtime.clone()
    }
}

impl FromRef<AppState> for Arc<Notify> {
    fn from_ref(state: &AppState) -> Self {
        state.runtime.proxy_sync_trigger.clone()
    }
}

impl FromRef<AppState> for Arc<LogManager> {
    fn from_ref(state: &AppState) -> Self {
        state.runtime.log_manager.clone()
    }
}

impl FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}
