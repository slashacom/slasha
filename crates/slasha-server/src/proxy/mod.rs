pub mod caddy_client;
pub mod container;
pub mod error;
pub mod sync;

pub use caddy_client::{CaddyClient, RouteEntry, Upstream};
pub use container::PROXY_NETWORK_NAME;
pub use error::ProxyError;
pub use sync::spawn_route_syncer;

pub type ProxyResult<T> = std::result::Result<T, ProxyError>;
