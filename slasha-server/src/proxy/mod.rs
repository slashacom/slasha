pub mod caddy_client;
pub mod config;
pub mod container;
pub mod reconcile;

pub use caddy_client::CaddyClient;
pub use config::RouteEntry;
pub use reconcile::reconcile;

use crate::error::ProxyError;
pub type ProxyResult<T> = std::result::Result<T, ProxyError>;
