use std::time::Duration;

use anyhow::{Context, Result};

use crate::{
    config::{DEFAULT_BASE_URL, GlobalConfig},
    token::get_auth_token,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct ApiClient {
    client: reqwest::Client,
    stream_client: reqwest::Client,
    base_url: String,
    git_host: Option<String>,
}

impl ApiClient {
    pub fn from_config() -> Result<Self> {
        let config = GlobalConfig::load()?;

        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("Failed to build HTTP client")?;

        let stream_client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .context("Failed to build streaming HTTP client")?;

        Ok(Self {
            client,
            stream_client,
            base_url: config
                .base_url
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            git_host: config.git_host,
        })
    }

    pub fn with_url_override(mut self, url: Option<String>) -> Self {
        if let Some(u) = url {
            self.base_url = u;
        }

        self
    }

    pub fn base_url(&self) -> &str {
        self.base_url.trim_end_matches('/')
    }

    pub fn git_host(&self) -> String {
        if let Some(h) = &self.git_host {
            return h.clone();
        }

        reqwest::Url::parse(&self.base_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_else(|| "localhost".to_string())
    }

    pub async fn get(&self, path: &str) -> Result<serde_json::Value> {
        self.send(self.client.get(self.url(path))).await
    }

    pub async fn get_stream(&self, path: &str) -> Result<reqwest::Response> {
        let res = self
            .apply_auth(self.stream_client.get(self.url(path)))?
            .send()
            .await
            .context("GET request failed")?;

        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("{}", format_error(status, &body));
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

    pub async fn patch<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<serde_json::Value> {
        self.send(self.client.patch(self.url(path)).json(body))
            .await
    }

    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        self.send(self.client.delete(self.url(path))).await
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url(), path.trim_start_matches('/'))
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

        let body_bytes = res.bytes().await.context("Failed to read response body")?;

        if status.is_success() {
            if body_bytes.is_empty() {
                return Ok(serde_json::Value::Null);
            }
            return serde_json::from_slice(&body_bytes).with_context(|| {
                let preview = String::from_utf8_lossy(&body_bytes);
                format!("Failed to parse JSON response: {}", preview)
            });
        }

        let body_text = String::from_utf8_lossy(&body_bytes).to_string();
        anyhow::bail!("{}", format_error(status, &body_text));
    }
}

fn format_error(status: reqwest::StatusCode, body: &str) -> String {
    let message = if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        value["error"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string())
    } else if body.trim().is_empty() {
        status.to_string()
    } else {
        body.trim().to_string()
    };

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return format!("{} (run `slasha login` to authenticate)", message);
    }

    message
}
