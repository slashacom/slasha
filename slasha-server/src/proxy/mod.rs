pub mod caddy_client;
pub mod container;
pub mod reconcile;

pub use caddy_client::{CaddyClient, RouteEntry};
pub use reconcile::spawn_reconciler;

use crate::error::ProxyError;
pub type ProxyResult<T> = std::result::Result<T, ProxyError>;
