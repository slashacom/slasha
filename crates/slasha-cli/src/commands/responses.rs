use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use slasha_db::models::logs::LogRecord;

#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct OkResponse;

impl<'de> Deserialize<'de> for OkResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let _ = serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(OkResponse)
    }
}

#[derive(Deserialize, Serialize)]
pub struct LogsResponse {
    pub logs: Vec<LogRecord>,
}

#[derive(Deserialize, Serialize)]
pub struct EnvVarsResponse {
    pub env_vars: HashMap<String, String>,
}
