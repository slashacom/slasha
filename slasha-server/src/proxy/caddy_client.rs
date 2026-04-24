use super::ProxyResult;
use crate::error::ProxyError;
use reqwest::Client;
use serde_json::Value;

#[derive(Default)]
pub struct CaddyClient {
    client: Client,
    admin_url: String,
}

impl CaddyClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            admin_url: "http://127.0.0.1:2019".to_string(),
        }
    }

    pub async fn load(&self, config: &Value) -> ProxyResult<()> {
        let url = format!("{}/load", self.admin_url);
        let res = self.client.post(&url).json(config).send().await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_else(|_| "unknown error".into());
            return Err(ProxyError::Caddy(body));
        }

        Ok(())
    }
}
