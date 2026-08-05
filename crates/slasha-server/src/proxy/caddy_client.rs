use reqwest::Client;
use serde_json::{Value, json};

use super::{ProxyError, error::ProxyResult};
use crate::state::Env;

/// Client for interacting with the Caddy admin API.
#[derive(Default, Clone)]
pub struct CaddyClient {
    client: Client,
}

/// Represents a backend upstream destination for proxying.
#[derive(Debug, Clone)]
pub struct Upstream {
    pub host: String,
    pub port: u16,
}

/// Represents a route mapping a domain to its backend upstreams.
#[derive(Debug)]
pub struct RouteEntry {
    pub domain: String,
    pub upstreams: Vec<Upstream>,
    pub tls_root_ca: Option<String>,
    pub tls_server_name: Option<String>,
}

impl CaddyClient {
    /// Builds a JSON configuration for Caddy routes based on the provided route entries.
    ///
    /// # Arguments
    ///
    /// * `routes` - List of route entries to configure ([`RouteEntry`]).
    /// * `internal_tls_domains` - List of internal domains requiring TLS.
    /// * `env` - Current application environment ([`Env`]).
    ///
    /// # Returns
    ///
    /// The generated Caddy configuration JSON.
    pub fn build_routes_config(
        routes: &[RouteEntry],
        internal_tls_domains: &[String],
        env: Env,
    ) -> Value {
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

                // on node setup we extract the remote ca and save it to the db
                // here the main server injects that ca so caddy trusts the remote node
                // we strip the pem headers because caddy expects raw base64 der
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

                    // when routing from the main server to a remote node, override sni so the
                    // remote node presents its self-signed cert instead of failing on a custom domain
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

        // forces self-signed certs for remote nodes (platform domain only)
        // main server proxies to them and trusts their ca
        // hitting the remote node directly on the platform domain causes a cert error
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

    /// Builds and applies a new routing configuration to a Caddy instance.
    ///
    /// # Arguments
    ///
    /// * `routes` - List of route entries to configure ([`RouteEntry`]).
    /// * `internal_tls_domains` - List of internal domains requiring TLS.
    /// * `env` - Current application environment ([`Env`]).
    /// * `base_url` - The base URL of the Caddy admin API.
    ///
    /// # Returns
    ///
    /// A [`ProxyResult`] indicating success or failure.
    pub async fn apply_routes(
        &self,
        routes: &[RouteEntry],
        internal_tls_domains: &[String],
        env: Env,
        base_url: &str,
    ) -> ProxyResult<()> {
        let config = Self::build_routes_config(routes, internal_tls_domains, env);
        self.apply_config(&config, base_url).await
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

        // hsts forces browsers to only use https. if sent during local dev,
        // a broken self-signed cert will permanently lock you out of localhost.
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
