use std::collections::HashMap;

use bollard::models::HostConfigLogConfig;

const MAX_SIZE: &str = "10m";
const MAX_FILE: &str = "3";

/// Returns the default JSON-file log driver configuration with rotation caps.
///
/// # Returns
///
/// A [`HostConfigLogConfig`](bollard::models::HostConfigLogConfig) struct.
pub fn default_log_config() -> HostConfigLogConfig {
    HostConfigLogConfig {
        typ: Some("json-file".to_string()),
        config: Some(HashMap::from([
            ("max-size".to_string(), MAX_SIZE.to_string()),
            ("max-file".to_string(), MAX_FILE.to_string()),
        ])),
    }
}
