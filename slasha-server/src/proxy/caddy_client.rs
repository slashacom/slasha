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
        let security_headers = Self::security_headers(env);
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
                        "handler": "headers",
                        "response": { "set": security_headers }
                    },
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

    fn security_headers(env: Env) -> Value {
        let mut headers = serde_json::Map::new();
        headers.insert("X-Content-Type-Options".into(), json!(["nosniff"]));
        headers.insert("X-Frame-Options".into(), json!(["DENY"]));
        headers.insert(
            "Referrer-Policy".into(),
            json!(["strict-origin-when-cross-origin"]),
        );
        headers.insert("Permissions-Policy".into(), json!(["interest-cohort=()"]));
        // HSTS only in production — sending it from a self-signed dev cert
        // pins browsers to a broken HTTPS state.
        if env.is_production() {
            headers.insert(
                "Strict-Transport-Security".into(),
                json!(["max-age=31536000; includeSubDomains"]),
            );
        }
        Value::Object(headers)
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
