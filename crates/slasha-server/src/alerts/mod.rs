mod delivery;
mod evaluation;
mod worker;

use std::time::Duration;

pub const CHECK_INTERVAL: Duration = Duration::from_secs(60);

pub use worker::spawn_alert_worker;
