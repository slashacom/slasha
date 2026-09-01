use std::time::Duration;

use anyhow::{Context, Result};

use crate::token::get_auth_token;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// HTTP API client for communicating with a remote Slasha server instance.
pub struct ApiClient {
    client: reqwest::Client,
    stream_client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    /// Constructs a new [`ApiClient`] targeting the specified base URL.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Normalized server HTTP base URL string.
    ///
    /// # Returns
    ///
    /// A configured [`ApiClient`] instance.
    pub fn new(base_url: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("Failed to build HTTP client")?;

        let stream_client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .context("Failed to build streaming HTTP client")?;

        let trimmed = base_url.trim();
        let base_url = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else {
            format!("http://{}", trimmed)
        };

        Ok(Self {
            client,
            stream_client,
            base_url,
        })
    }

    /// Returns the target base URL with trailing slashes trimmed.
    ///
    /// # Returns
    ///
    /// String slice referencing the normalized base URL.
    pub fn base_url(&self) -> &str {
        self.base_url.trim_end_matches('/')
    }

    /// Returns the host string derived from base_url for Git SSH operations.
    ///
    /// # Returns
    ///
    /// The resolved Git host string.
    pub fn git_host(&self) -> String {
        reqwest::Url::parse(&self.base_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_else(|| "localhost".to_string())
    }

    /// Executes an HTTP GET request and deserializes the JSON response body.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative API endpoint path.
    ///
    /// # Returns
    ///
    /// Deserialized target response type `T`.
    pub async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.send(self.client.get(self.url(path))).await
    }

    /// Executes an HTTP GET request returning a raw streaming response stream.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative API endpoint path.
    ///
    /// # Returns
    ///
    /// The raw [`reqwest::Response`] stream.
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

    /// Executes an HTTP POST request sending a JSON body payload.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative API endpoint path.
    /// * `body` - Serializable payload reference.
    ///
    /// # Returns
    ///
    /// Deserialized target response type `T`.
    pub async fn post<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.send(self.client.post(self.url(path)).json(body)).await
    }

    /// Executes an HTTP PUT request sending a JSON body payload.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative API endpoint path.
    /// * `body` - Serializable payload reference.
    ///
    /// # Returns
    ///
    /// Deserialized target response type `T`.
    pub async fn put<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.send(self.client.put(self.url(path)).json(body)).await
    }

    /// Executes an HTTP PATCH request sending a JSON body payload.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative API endpoint path.
    /// * `body` - Serializable payload reference.
    ///
    /// # Returns
    ///
    /// Deserialized target response type `T`.
    #[allow(dead_code)]
    pub async fn patch<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.send(self.client.patch(self.url(path)).json(body))
            .await
    }

    /// Executes an HTTP DELETE request.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative API endpoint path.
    ///
    /// # Returns
    ///
    /// Deserialized target response type `T`.
    pub async fn delete<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.send(self.client.delete(self.url(path))).await
    }

    /// Constructs a fully qualified URL for a relative API endpoint path.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative endpoint path slice.
    ///
    /// # Returns
    ///
    /// Fully qualified URL string.
    pub fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url(), path.trim_start_matches('/'))
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder> {
        if let Some(token) = get_auth_token(&self.base_url)? {
            return Ok(req.bearer_auth(token));
        }

        Ok(req)
    }

    async fn send<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T> {
        let res = self
            .apply_auth(req)?
            .send()
            .await
            .context("Request failed")?;

        let status = res.status();
        if status == reqwest::StatusCode::NO_CONTENT {
            return serde_json::from_str("null").context("Failed to parse null response");
        }

        let body_bytes = res.bytes().await.context("Failed to read response body")?;

        if status.is_success() {
            if body_bytes.is_empty() {
                return serde_json::from_str("null").context("Failed to parse empty response");
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
        return format!("{} (run `slasha auth login` to authenticate)", message);
    }

    message
}
