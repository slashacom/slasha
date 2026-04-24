use serde_json::{Value, json};

pub struct RouteEntry {
    pub domain: String,
    pub upstream_port: u16,
}

pub fn build_config(routes: &[RouteEntry]) -> Value {
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