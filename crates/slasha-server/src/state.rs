use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::extract::FromRef;
use slasha_db::{DbPool, DuckdbPool, repos::github_app_config::GithubAppConfigRepo};
use tokio::sync::{Notify, RwLock};

use crate::{
    connections::GithubClient,
    docker::DockerRegistry,
    logs::LogManager,
    node_connection_manager::NodeConnectionManager,
    proxy::CaddyClient,
    utils::{self},
};

#[derive(Clone)]
pub struct Clients {
    pub node_connection_manager: Arc<NodeConnectionManager>,
    pub docker_registry: DockerRegistry,
    pub caddy_client: CaddyClient,
    pub github: Arc<RwLock<Option<GithubClient>>>,
}

impl Clients {
    pub fn new(github: Option<GithubClient>, nodes_dir: PathBuf) -> Self {
        let node_connection_manager = Arc::new(NodeConnectionManager::new(nodes_dir));

        Self {
            docker_registry: DockerRegistry::new(node_connection_manager.clone()),
            node_connection_manager,
            caddy_client: CaddyClient::default(),
            github: Arc::new(RwLock::new(github)),
        }
    }
}

#[derive(Clone)]
pub struct Storage {
    pub db_pool: DbPool,
    pub duckdb_pool: DuckdbPool,
    pub repos_dir: PathBuf,
}

impl Storage {
    pub fn new(
        db_path: &std::path::Path,
        duckdb_path: &std::path::Path,
        repos_dir: PathBuf,
    ) -> anyhow::Result<Self> {
        let db_str = db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid DB path"))?;
        let duckdb_str = duckdb_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid DuckDB path"))?;

        let db_pool = slasha_db::create_pool_with_max_size(db_str, 10)?;
        let duckdb_pool = slasha_db::create_duckdb_pool_with_max_size(duckdb_str, 10)?;

        Ok(Self {
            db_pool,
            duckdb_pool,
            repos_dir,
        })
    }
}

#[derive(Clone)]
pub struct Runtime {
    pub log_manager: Arc<LogManager>,
    pub proxy_sync_trigger: Arc<Notify>,
    pub scaling_locks: Arc<dashmap::DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    pub connection_sync_locks: Arc<dashmap::DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    pub deployment_tasks: Arc<dashmap::DashMap<String, tokio_util::sync::CancellationToken>>,
    pub migrating_apps: Arc<dashmap::DashSet<String>>,
}

impl Runtime {
    pub async fn new(logs_dir: &Path, proxy_sync_trigger: Arc<Notify>) -> anyhow::Result<Self> {
        Ok(Self {
            log_manager: Arc::new(LogManager::new(utils::ensure_dir(logs_dir))),
            proxy_sync_trigger,
            scaling_locks: Arc::new(dashmap::DashMap::new()),
            connection_sync_locks: Arc::new(dashmap::DashMap::new()),
            deployment_tasks: Arc::new(dashmap::DashMap::new()),
            migrating_apps: Arc::new(dashmap::DashSet::new()),
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

    pub async fn github_client(&self) -> Option<GithubClient> {
        self.clients.github.read().await.clone()
    }

    pub async fn reload_github_client(&self) -> anyhow::Result<()> {
        let config = GithubAppConfigRepo::get(&self.storage.db_pool).await?;
        let client = config.as_ref().map(GithubClient::from_config).transpose()?;
        *self.clients.github.write().await = client;
        Ok(())
    }

    pub async fn clear_github_client(&self) {
        *self.clients.github.write().await = None;
    }
}

impl FromRef<AppState> for Clients {
    fn from_ref(state: &AppState) -> Self {
        state.clients.clone()
    }
}

impl FromRef<AppState> for CaddyClient {
    fn from_ref(state: &AppState) -> Self {
        state.clients.caddy_client.clone()
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

impl FromRef<AppState> for DuckdbPool {
    fn from_ref(state: &AppState) -> Self {
        state.storage.duckdb_pool.clone()
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
