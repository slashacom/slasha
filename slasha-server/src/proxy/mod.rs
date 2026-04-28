pub mod caddy_client;
pub mod container;
pub mod error;
pub mod reconcile;

pub use caddy_client::{CaddyClient, RouteEntry};
pub use error::ProxyError;
pub use reconcile::spawn_reconciler;

pub type ProxyResult<T> = std::result::Result<T, ProxyError>;
