use reqwest::Client;
use serde_json::{Value, json};

use super::{ProxyError, ProxyResult};
use crate::state::Env;

#[derive(Default, Clone)]
pub struct CaddyClient {
    client: Client,
}

#[derive(Debug)]
pub struct Upstream {
    pub host: String,
    pub port: u16,
}

#[derive(Debug)]
pub struct RouteEntry {
    pub domain: String,
    pub upstreams: Vec<Upstream>,
}

impl CaddyClient {
    pub async fn apply_routes(&self, routes: &[RouteEntry], env: Env) -> ProxyResult<()> {
        let config = Self::build_config(routes, env);
        self.apply_config(&config).await
    }

    fn build_config(routes: &[RouteEntry], env: Env) -> Value {
        let security_headers = Self::security_headers(env);

        let caddy_routes: Vec<Value> = routes
            .iter()
            .map(|entry| {
                let upstream_objects: Vec<Value> = entry
                    .upstreams
                    .iter()
                    .map(|u| json!({ "dial": format!("{}:{}", u.host, u.port) }))
                    .collect();

                json!({
                    "match": [{ "host": [entry.domain] }],
                    "handle": [
                        {
                            "handler": "headers",
                            "response": { "set": security_headers }
                        },
                        {
                            "handler": "reverse_proxy",
                            "upstreams": upstream_objects
                        }
                    ]
                })
            })
            .collect();

        let mut server = json!({
            "listen": [":80", ":443"],
            "routes": caddy_routes
        });

        if !env.is_production() {
            server["automatic_https"] = json!({ "disable_redirects": true });
        }

        let mut apps = json!({
            "http": {
                "servers": {
                    "srv0": server
                }
            }
        });

        if !env.is_production() {
            apps["tls"] = json!({
                "automation": {
                    "policies": [{ "issuers": [{ "module": "internal" }] }]
                }
            });
        }

        json!({
            "admin": { "listen": "0.0.0.0:2019" },
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

    async fn apply_config(&self, config: &Value) -> ProxyResult<()> {
        let res = self
            .client
            .post("http://127.0.0.1:2019/load")
            .json(config)
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await?;
            return Err(ProxyError::Caddy(body));
        }

        Ok(())
    }
}
