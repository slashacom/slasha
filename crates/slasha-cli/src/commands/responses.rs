use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use slasha_db::models::logs::LogRecord;

#[derive(Deserialize, Serialize)]
pub struct OkResponse {
    pub ok: bool,
}

#[derive(Deserialize, Serialize)]
pub struct LogsResponse {
    pub logs: Vec<LogRecord>,
}

#[derive(Deserialize, Serialize)]
pub struct EnvVarsResponse {
    pub env_vars: HashMap<String, String>,
}
