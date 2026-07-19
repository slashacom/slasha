use reqwest::Client;
use serde_json::{Value, json};

use super::{ProxyError, ProxyResult};
use crate::state::Env;

#[derive(Default, Clone)]
pub struct CaddyClient {
    client: Client,
}

#[derive(Debug, Clone)]
pub struct Upstream {
    pub host: String,
    pub port: u16,
}

#[derive(Debug)]
pub struct RouteEntry {
    pub domain: String,
    pub upstreams: Vec<Upstream>,
    pub tls_root_ca: Option<String>,
    pub tls_server_name: Option<String>,
}

impl CaddyClient {
    pub fn build_routes_config(
        &self,
        routes: &[RouteEntry],
        internal_tls_domains: &[String],
        env: Env,
    ) -> Value {
        Self::build_config(routes, internal_tls_domains, env)
    }

    pub async fn apply_routes(
        &self,
        routes: &[RouteEntry],
        internal_tls_domains: &[String],
        env: Env,
        base_url: &str,
    ) -> ProxyResult<()> {
        let config = Self::build_config(routes, internal_tls_domains, env);
        self.apply_config(&config, base_url).await
    }

    fn build_config(routes: &[RouteEntry], internal_tls_domains: &[String], env: Env) -> Value {
        let security_headers = Self::security_headers(env);

        let caddy_routes: Vec<Value> = routes
            .iter()
            .map(|entry| {
                let upstream_objects: Vec<Value> = entry
                    .upstreams
                    .iter()
                    .map(|u| json!({ "dial": format!("{}:{}", u.host, u.port) }))
                    .collect();

                let mut reverse_proxy = json!({
                    "handler": "reverse_proxy",
                    "upstreams": upstream_objects
                });

                // When proxying to a remote app node over HTTPS (using internal self-signed certs),
                // we strip the PEM headers and pass the raw base64 DER to Caddy so it trusts the node's CA.
                if let Some(root_ca) = &entry.tls_root_ca {
                    let base64_der = root_ca
                        .replace("-----BEGIN CERTIFICATE-----", "")
                        .replace("-----END CERTIFICATE-----", "")
                        .replace(['\n', '\r'], "")
                        .trim()
                        .to_string();

                    let mut tls_config = json!({
                        "ca": {
                            "provider": "inline",
                            "trusted_ca_certs": [base64_der]
                        }
                    });

                    // override sni server name to match the remote node's wildcard certificate
                    if let Some(server_name) = &entry.tls_server_name {
                        tls_config["server_name"] = json!(server_name);
                    }

                    reverse_proxy["transport"] = json!({
                        "protocol": "http",
                        "tls": tls_config
                    });
                }

                json!({
                    "match": [{ "host": [entry.domain] }],
                    "handle": [
                        {
                            "handler": "headers",
                            "response": { "set": security_headers }
                        },
                        reverse_proxy
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

        if !env.is_production() || !internal_tls_domains.is_empty() {
            let mut policies = Vec::new();
            if !env.is_production() {
                policies.push(json!({ "issuers": [{ "module": "internal" }] }));
            } else if !internal_tls_domains.is_empty() {
                policies.push(json!({
                    "subjects": internal_tls_domains,
                    "issuers": [{ "module": "internal" }]
                }));
            }

            apps["tls"] = json!({
                "automation": {
                    "policies": policies
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

    async fn apply_config(&self, config: &Value, base_url: &str) -> ProxyResult<()> {
        let url = format!("{}/load", base_url.trim_end_matches('/'));
        let res = self.client.post(&url).json(config).send().await?;

        if !res.status().is_success() {
            let body = res.text().await?;
            return Err(ProxyError::Caddy(body));
        }

        Ok(())
    }
}
