use anyhow::{Context, Result};

use crate::config::Config;

pub struct ApiClient {
    inner: reqwest::Client,
    base_url: String,
    auth_token: Option<String>,
}

impl ApiClient {
    pub fn from_config() -> Result<Self> {
        let config = Config::load().context("failed to load config")?;
        let inner = reqwest::Client::builder()
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            inner,
            base_url: config.base_url,
            auth_token: config.auth_token,
        })
    }

    pub fn with_url_override(mut self, url: Option<String>) -> Self {
        if let Some(u) = url {
            self.base_url = u;
        }
        self
    }

    pub async fn get(&self, path: &str) -> Result<serde_json::Value> {
        self.send(self.inner.get(self.url(path)), path).await
    }

    pub async fn get_stream(&self, path: &str) -> Result<reqwest::Response> {
        self.apply_auth(self.inner.get(self.url(path)))
            .send()
            .await
            .with_context(|| format!("GET {} failed", path))
    }

    pub async fn post<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<serde_json::Value> {
        self.send(self.inner.post(self.url(path)).json(body), path)
            .await
    }

    pub async fn put<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<serde_json::Value> {
        self.send(self.inner.put(self.url(path)).json(body), path)
            .await
    }

    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        self.send(self.inner.delete(self.url(path)), path).await
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth_token {
            Some(token) => req.bearer_auth(token),
            None => req,
        }
    }

    async fn send(&self, req: reqwest::RequestBuilder, path: &str) -> Result<serde_json::Value> {
        let res = self
            .apply_auth(req)
            .send()
            .await
            .with_context(|| format!("{} failed", path))?;
        Self::parse_response(res).await
    }

    async fn parse_response(res: reqwest::Response) -> Result<serde_json::Value> {
        let status = res.status();
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(serde_json::Value::Null);
        }
        let json: serde_json::Value = res.json().await.context("failed to parse response body")?;
        if !status.is_success() {
            let msg = json["error"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| json.to_string());
            anyhow::bail!("{}", msg);
        }
        Ok(json)
    }
}

pub fn client() -> Result<ApiClient> {
    ApiClient::from_config()
}
