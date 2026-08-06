use std::{path::PathBuf, sync::Arc};

use axum::extract::FromRef;
use slasha_db::{DbPool, DuckdbPool, repos::github_app_config::GithubAppConfigRepo};
use tokio::sync::{Notify, RwLock};

use crate::{
    connections::GithubClient, logs::LogBus, node_registry::NodeRegistry,
    operations::OperationRegistry, proxy::CaddyClient,
};

/// Collection of shared external system clients and connections.
#[derive(Clone)]
pub struct Clients {
    pub caddy_client: CaddyClient,
    pub github: Arc<RwLock<Option<GithubClient>>>,
}

impl Clients {
    /// Constructs a new [`Clients`] container holding connections and API clients.
    ///
    /// # Arguments
    ///
    /// * `github` - Optional pre-configured GitHub API client ([`GithubClient`]).
    ///
    /// # Returns
    ///
    /// A new [`Clients`] instance.
    pub fn new(github: Option<GithubClient>) -> Self {
        Self {
            caddy_client: CaddyClient::default(),
            github: Arc::new(RwLock::new(github)),
        }
    }
}

/// Persistent database connection pools and repository storage paths.
#[derive(Clone)]
pub struct Storage {
    pub db_pool: DbPool,
    pub duckdb_pool: DuckdbPool,
    pub repos_dir: PathBuf,
}

impl Storage {
    /// Initializes database connection pools and creates a [`Storage`] instance.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the primary SQLite database file.
    /// * `duckdb_path` - Path to the DuckDB metrics database file.
    /// * `repos_dir` - Root directory where app source repositories are cloned.
    ///
    /// # Returns
    ///
    /// An [`anyhow::Result`] containing the initialized [`Storage`].
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

/// In-memory runtime state.
#[derive(Clone)]
pub struct Runtime {
    pub log_bus: LogBus,
    pub proxy_sync_trigger: Arc<Notify>,
    pub operations: OperationRegistry,
}

impl Runtime {
    /// Constructs a new [`Runtime`] state instance holding log handles and operation registries.
    ///
    /// # Arguments
    ///
    /// * `duckdb_pool` - DuckDB connection pool for the log bus ([`DuckdbPool`]).
    /// * `proxy_sync_trigger` - Shared notification trigger for proxy route sync ([`Notify`]).
    ///
    /// # Returns
    ///
    /// An [`anyhow::Result`] containing the initialized [`Runtime`].
    pub async fn new(
        duckdb_pool: DuckdbPool,
        proxy_sync_trigger: Arc<Notify>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            log_bus: LogBus::new(duckdb_pool),
            proxy_sync_trigger,
            operations: OperationRegistry::new(),
        })
    }
}

/// Server execution environment mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Env {
    Development,
    Production,
}

impl Env {
    /// Parses an environment string or defaults to [`Env::Development`].
    ///
    /// # Arguments
    ///
    /// * `s` - Environment string.
    ///
    /// # Returns
    ///
    /// An [`Env`] variant.
    pub fn from_str_or_default(s: &str) -> Self {
        match s {
            "production" => Env::Production,
            _ => Env::Development,
        }
    }

    /// Returns whether the environment is running in production mode.
    ///
    /// # Returns
    ///
    /// `true` if production mode, otherwise `false`.
    pub fn is_production(self) -> bool {
        matches!(self, Env::Production)
    }
}

/// Global configuration options parsed from system environment variables.
#[derive(Clone)]
pub struct Config {
    pub env: Env,
    pub jwt_secret: String,
    pub platform_domain: String,
    pub port: u16,
}

impl Config {
    /// Creates a new [`Config`] instance from platform settings.
    ///
    /// # Arguments
    ///
    /// * `env` - Execution environment mode ([`Env`]).
    /// * `jwt_secret` - JWT signing secret key string.
    /// * `platform_domain` - Base platform domain string.
    /// * `port` - Listening HTTP port number (`u16`).
    ///
    /// # Returns
    ///
    /// A new [`Config`] instance.
    pub fn new(env: Env, jwt_secret: String, platform_domain: String, port: u16) -> Self {
        Self {
            env,
            jwt_secret,
            platform_domain,
            port,
        }
    }
}

/// Top-level application state shared across all HTTP route handlers.
#[derive(Clone)]
pub struct AppState {
    pub node_registry: NodeRegistry,
    pub clients: Clients,
    pub storage: Storage,
    pub runtime: Runtime,
    pub config: Config,
}

impl AppState {
    /// Constructs a new [`AppState`] instance shared across HTTP handlers.
    ///
    /// # Arguments
    ///
    /// * `config` - Global configuration ([`Config`]).
    /// * `node_registry` - Node registry ([`NodeRegistry`]).
    /// * `clients` - External clients container ([`Clients`]).
    /// * `storage` - Database storage pools ([`Storage`]).
    /// * `runtime` - Runtime state ([`Runtime`]).
    ///
    /// # Returns
    ///
    /// A new [`AppState`] instance.
    pub fn new(
        config: Config,
        node_registry: NodeRegistry,
        clients: Clients,
        storage: Storage,
        runtime: Runtime,
    ) -> Self {
        Self {
            node_registry,
            clients,
            storage,
            runtime,
            config,
        }
    }

    /// Asynchronously retrieves the current GitHub client if initialized.
    ///
    /// # Returns
    ///
    /// Option containing a [`GithubClient`].
    pub async fn github_client(&self) -> Option<GithubClient> {
        self.clients.github.read().await.clone()
    }

    /// Reloads the GitHub client configuration from the database repository.
    ///
    /// # Returns
    ///
    /// An [`anyhow::Result`] indicating reload status.
    pub async fn reload_github_client(&self) -> anyhow::Result<()> {
        let config = GithubAppConfigRepo::get(&self.storage.db_pool).await?;
        let client = config.as_ref().map(GithubClient::from_config).transpose()?;
        *self.clients.github.write().await = client;
        Ok(())
    }

    /// Clears the active GitHub client instance.
    pub async fn clear_github_client(&self) {
        *self.clients.github.write().await = None;
    }
}

impl FromRef<AppState> for NodeRegistry {
    fn from_ref(state: &AppState) -> Self {
        state.node_registry.clone()
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

impl FromRef<AppState> for LogBus {
    fn from_ref(state: &AppState) -> Self {
        state.runtime.log_bus.clone()
    }
}

impl FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}
