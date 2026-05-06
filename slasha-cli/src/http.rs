use anyhow::{Context, Result};

use crate::{config::Config, token::get_auth_token};

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    pub fn from_config() -> Result<Self> {
        let config = Config::load()?;
        let client = reqwest::Client::builder()
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            base_url: config
                .base_url
                .unwrap_or("http://localhost:3000".to_string()),
        })
    }

    pub fn with_url_override(mut self, url: Option<String>) -> Self {
        if let Some(u) = url {
            self.base_url = u;
        }

        self
    }

    pub async fn get(&self, path: &str) -> Result<serde_json::Value> {
        self.send(self.client.get(self.url(path))).await
    }

    pub async fn get_stream(&self, path: &str) -> Result<reqwest::Response> {
        let res = self
            .apply_auth(self.client.get(self.url(path)))?
            .send()
            .await
            .context("GET request failed")?;

        if !res.status().is_success() {
            let body: serde_json::Value =
                res.json().await.context("Failed to parse response body")?;

            let msg = body["error"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| body.to_string());

            anyhow::bail!("{}", msg);
        }

        Ok(res)
    }

    pub async fn post<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<serde_json::Value> {
        self.send(self.client.post(self.url(path)).json(body)).await
    }

    pub async fn put<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<serde_json::Value> {
        self.send(self.client.put(self.url(path)).json(body)).await
    }

    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        self.send(self.client.delete(self.url(path))).await
    }

    pub fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder> {
        if let Some(token) = get_auth_token()? {
            return Ok(req.bearer_auth(token));
        }

        Ok(req)
    }

    async fn send(&self, req: reqwest::RequestBuilder) -> Result<serde_json::Value> {
        let res = self
            .apply_auth(req)?
            .send()
            .await
            .context("Request failed")?;

        let status = res.status();
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(serde_json::Value::Null);
        }

        let body: serde_json::Value = res.json().await.context("Failed to parse response body")?;

        if !status.is_success() {
            let msg = body["error"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| body.to_string());

            anyhow::bail!("{}", msg);
        }

        Ok(body)
    }
}
