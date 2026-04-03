use crate::config::Config;
use anyhow::Context;
use reqwest::{Client, RequestBuilder, Response};
use serde::Serialize;
use std::sync::OnceLock;

static HTTP: OnceLock<ApiClient> = OnceLock::new();

pub fn client() -> anyhow::Result<&'static ApiClient> {
    if let Some(c) = HTTP.get() {
        return Ok(c);
    }

    let api = ApiClient::from_config()?;
    Ok(HTTP.get_or_init(|| api))
}

pub struct ApiClient {
    inner: Client,
    base_url: String,
    auth_token: Option<String>,
}

impl ApiClient {
    fn from_config() -> anyhow::Result<Self> {
        let config = Config::load().context("failed to load config")?;

        let base_url = config
            .base_url
            .unwrap_or_else(|| "http://localhost:3000".to_string());

        let inner = Client::builder()
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            inner,
            base_url,
            auth_token: config.auth_token,
        })
    }

    pub async fn get(&self, path: &str) -> anyhow::Result<Response> {
        self.send(self.inner.get(self.url(path))).await
    }

    pub async fn post<B: Serialize>(&self, path: &str, body: &B) -> anyhow::Result<Response> {
        self.send(self.inner.post(self.url(path)).json(body)).await
    }

    pub async fn patch<B: Serialize>(&self, path: &str, body: &B) -> anyhow::Result<Response> {
        self.send(self.inner.patch(self.url(path)).json(body)).await
    }

    pub async fn put<B: Serialize>(&self, path: &str, body: &B) -> anyhow::Result<Response> {
        self.send(self.inner.put(self.url(path)).json(body)).await
    }

    pub async fn delete(&self, path: &str) -> anyhow::Result<Response> {
        self.send(self.inner.delete(self.url(path))).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn send(&self, builder: RequestBuilder) -> anyhow::Result<Response> {
        let builder = match &self.auth_token {
            Some(token) => builder.bearer_auth(token),
            None => builder,
        };

        builder.send().await.context("HTTP request failed")
    }
}
