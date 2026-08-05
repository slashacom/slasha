use std::{fmt, str};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, TS)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[ts(export, export_to = "./logs.ts")]
pub enum ResourceKind {
    Deployment,
    Service,
    Cron,
    Node,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, TS)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[ts(export, export_to = "./logs.ts")]
pub enum LogStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "./logs.ts")]
pub enum LogPrefix {
    System,
    Web(u32),
    Worker(u32),
    Service,
    Custom(String),
}

impl fmt::Display for LogPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => f.write_str("system"),
            Self::Web(idx) => write!(f, "web.{idx}"),
            Self::Worker(idx) => write!(f, "worker.{idx}"),
            Self::Service => f.write_str("service"),
            Self::Custom(value) => f.write_str(value),
        }
    }
}

impl str::FromStr for LogPrefix {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value_lower = value.to_ascii_lowercase();

        let parse_index = |prefix: &str| {
            value_lower
                .strip_prefix(prefix)
                .and_then(|index| index.parse::<u32>().ok())
        };

        let prefix = match value_lower.as_str() {
            "system" => Self::System,
            "service" => Self::Service,

            _ => match parse_index("web.") {
                Some(index) => Self::Web(index),
                None => match parse_index("worker.") {
                    Some(index) => Self::Worker(index),
                    None => Self::Custom(value.to_string()),
                },
            },
        };

        Ok(prefix)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./logs.ts")]
pub struct LogRecord {
    pub id: String,
    pub timestamp: NaiveDateTime,
    pub resource_kind: ResourceKind,
    pub resource_id: String,
    pub app_id: Option<String>,
    pub prefix: Option<LogPrefix>,
    pub stream: LogStream,
    pub message: String,
}
