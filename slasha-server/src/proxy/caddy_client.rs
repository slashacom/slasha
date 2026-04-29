use std::sync::Arc;

use reqwest::Client;
use serde_json::{Value, json};

use super::{ProxyError, ProxyResult};
use crate::state::Env;

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
    pub async fn sync_routes(&self, routes: &[RouteEntry], env: Env) -> ProxyResult<()> {
        let config = Self::build_config(routes, env);
        self.load(&config).await
    }

    fn build_config(routes: &[RouteEntry], env: Env) -> Value {
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

        let mut apps = json!({
            "http": {
                "servers": {
                    "srv0": {
                        "listen": [":80", ":443"],
                        "routes": caddy_routes
                    }
                }
            }
        });

        if !env.is_production() {
            apps["tls"] = json!({
                "automation": {
                    "policies": [
                        { "issuers": [{ "module": "internal" }] }
                    ]
                }
            });
        }

        json!({
            "admin": { "listen": "127.0.0.1:2019" },
            "apps": apps,
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
