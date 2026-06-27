pub mod app;
pub mod domain;
pub mod server;
pub mod service;
pub mod utils;

use std::time::Duration;

pub const COLLECT_INTERVAL: Duration = Duration::from_secs(10);

/// Domain DNS/TLS health is far slower-moving than resource metrics and each
/// check makes a live network probe, so it runs on a much longer cadence.
pub const DOMAIN_CHECK_INTERVAL: Duration = Duration::from_secs(15 * 60);
