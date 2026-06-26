pub mod app;
pub mod server;
pub mod service;
pub mod utils;

use std::time::Duration;

pub const COLLECT_INTERVAL: Duration = Duration::from_secs(10);
