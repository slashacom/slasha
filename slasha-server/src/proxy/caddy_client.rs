use super::ProxyResult;
use crate::error::ProxyError;
use reqwest::Client;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone)]
pub struct CaddyClient {
    client: Client,
    admin_url: Arc<str>,
}

pub struct RouteEntry {
    pub domain: String,
    pub upstream_port: u16,
}

impl CaddyClient {
    pub async fn sync_routes(&self, routes: &[RouteEntry]) -> ProxyResult<()> {
        let config = Self::build_config(routes);
        self.load(&config).await
    }

    fn build_config(routes: &[RouteEntry]) -> Value {
        let mut caddy_routes = Vec::new();

        for entry in routes {
            caddy_routes.push(json!({
                "match": [
                    {
                        "host": [entry.domain]
                    }
                ],
                "handle": [
                    {
                        "handler": "reverse_proxy",
                        "upstreams": [
                            {
                                "dial": format!("127.0.0.1:{}", entry.upstream_port)
                            }
                        ]
                    }
                ]
            }));
        }

        json!({
            "admin": {
                "listen": "127.0.0.1:2019"
            },
            "apps": {
                "http": {
                    "servers": {
                        "srv0": {
                            "listen": [":80", ":443"],
                            "routes": caddy_routes
                        }
                    }
                },
                "tls": {
                    "automation": {
                        "policies": [
                            {
                                "issuers": [
                                    {
                                        "module": "internal"
                                    }
                                ]
                            }
                        ]
                    }
                }
            }
        })
    }

    async fn load(&self, config: &Value) -> ProxyResult<()> {
        let url = format!("{}/load", self.admin_url);
        let res = self.client.post(&url).json(config).send().await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_else(|_| "unknown error".into());
            return Err(ProxyError::Caddy(body));
        }

        Ok(())
    }
}

impl Default for CaddyClient {
    fn default() -> Self {
        Self {
            client: Client::new(),
            admin_url: Arc::from("http://127.0.0.1:2019"),
        }
    }
}
