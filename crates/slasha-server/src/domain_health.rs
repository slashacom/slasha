use std::{collections::HashSet, net::IpAddr, time::Duration};

use futures_util::future::join_all;
use reqwest::tls::TlsInfo;
use serde::Serialize;
use x509_parser::prelude::*;

use crate::state::Config;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Pending,
    Error,
    Unknown,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DnsStatus {
    Ok,
    Proxied,
    Mismatch,
    Unresolved,
    Unknown,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TlsStatus {
    Active,
    Pending,
    Expired,
    Unreachable,
    Unknown,
}

#[derive(Serialize)]
pub struct DnsHealth {
    pub status: DnsStatus,
    pub resolved_ips: Vec<String>,
    pub expected_ips: Vec<String>,
    pub proxy: Option<&'static str>,
}

#[derive(Serialize)]
pub struct TlsHealth {
    pub status: TlsStatus,
    pub issuer: Option<String>,
    pub expires_at: Option<String>,
    pub days_until_expiry: Option<i64>,
}

#[derive(Serialize)]
pub struct DomainHealth {
    pub domain: String,
    pub status: HealthStatus,
    pub dns: DnsHealth,
    pub tls: TlsHealth,
}

pub async fn check_domains(domains: Vec<String>, config: &Config) -> Vec<DomainHealth> {
    let expected_ips = resolve_ips(&config.platform_domain).await;

    let checks = domains
        .into_iter()
        .map(|domain| check_one(domain, &expected_ips));

    join_all(checks).await
}

async fn check_one(domain: String, expected_ips: &[IpAddr]) -> DomainHealth {
    let (dns, tls) = tokio::join!(check_dns(&domain, expected_ips), check_tls(&domain));

    let status = overall_status(&dns.status, &tls.status);

    DomainHealth {
        domain,
        status,
        dns,
        tls,
    }
}

fn overall_status(dns: &DnsStatus, tls: &TlsStatus) -> HealthStatus {
    match dns {
        DnsStatus::Unresolved | DnsStatus::Mismatch => return HealthStatus::Error,
        DnsStatus::Ok | DnsStatus::Proxied | DnsStatus::Unknown => {}
    }

    match tls {
        TlsStatus::Active => HealthStatus::Healthy,
        TlsStatus::Pending => HealthStatus::Pending,
        TlsStatus::Expired | TlsStatus::Unreachable => HealthStatus::Error,
        TlsStatus::Unknown => HealthStatus::Unknown,
    }
}

async fn check_dns(domain: &str, expected_ips: &[IpAddr]) -> DnsHealth {
    let resolved = resolve_ips(domain).await;
    let expected_set: HashSet<&IpAddr> = expected_ips.iter().collect();

    let proxy = detect_proxy(&resolved);

    let status = if resolved.is_empty() {
        DnsStatus::Unresolved
    } else if expected_ips.is_empty() {
        DnsStatus::Unknown
    } else if resolved.iter().any(|ip| expected_set.contains(ip)) {
        DnsStatus::Ok
    } else if proxy.is_some() {
        DnsStatus::Proxied
    } else {
        DnsStatus::Mismatch
    };

    DnsHealth {
        status,
        resolved_ips: resolved.iter().map(|ip| ip.to_string()).collect(),
        expected_ips: expected_ips.iter().map(|ip| ip.to_string()).collect(),
        proxy,
    }
}

// Published edge ranges of proxies that sit in front of the origin, so the
// domain resolving to them is expected rather than a misconfiguration.
// Cloudflare's are from https://www.cloudflare.com/ips
const PROXY_RANGES: &[(&str, &str)] = &[
    ("173.245.48.0/20", "Cloudflare"),
    ("103.21.244.0/22", "Cloudflare"),
    ("103.22.200.0/22", "Cloudflare"),
    ("103.31.4.0/22", "Cloudflare"),
    ("141.101.64.0/18", "Cloudflare"),
    ("108.162.192.0/18", "Cloudflare"),
    ("190.93.240.0/20", "Cloudflare"),
    ("188.114.96.0/20", "Cloudflare"),
    ("197.234.240.0/22", "Cloudflare"),
    ("198.41.128.0/17", "Cloudflare"),
    ("162.158.0.0/15", "Cloudflare"),
    ("104.16.0.0/13", "Cloudflare"),
    ("104.24.0.0/14", "Cloudflare"),
    ("172.64.0.0/13", "Cloudflare"),
    ("131.0.72.0/22", "Cloudflare"),
    ("2400:cb00::/32", "Cloudflare"),
    ("2606:4700::/32", "Cloudflare"),
    ("2803:f800::/32", "Cloudflare"),
    ("2405:b500::/32", "Cloudflare"),
    ("2405:8100::/32", "Cloudflare"),
    ("2a06:98c0::/29", "Cloudflare"),
    ("2c0f:f248::/32", "Cloudflare"),
];

fn detect_proxy(resolved: &[IpAddr]) -> Option<&'static str> {
    if resolved.is_empty() {
        return None;
    }

    let mut proxy = None;
    for ip in resolved {
        let name = PROXY_RANGES
            .iter()
            .find(|(cidr, _)| ip_in_cidr(ip, cidr))
            .map(|(_, name)| *name)?;

        if *proxy.get_or_insert(name) != name {
            return None;
        }
    }

    proxy
}

fn ip_in_cidr(ip: &IpAddr, cidr: &str) -> bool {
    let Some((network, prefix)) = cidr.split_once('/') else {
        return false;
    };
    let Ok(prefix) = prefix.parse::<u32>() else {
        return false;
    };
    let Ok(network) = network.parse::<IpAddr>() else {
        return false;
    };

    match (ip, network) {
        (IpAddr::V4(ip), IpAddr::V4(network)) if prefix <= 32 => {
            let mask = u32::MAX.checked_shl(32 - prefix).unwrap_or(0);
            u32::from(*ip) & mask == u32::from(network) & mask
        }
        (IpAddr::V6(ip), IpAddr::V6(network)) if prefix <= 128 => {
            let mask = u128::MAX.checked_shl(128 - prefix).unwrap_or(0);
            u128::from(*ip) & mask == u128::from(network) & mask
        }
        _ => false,
    }
}

async fn resolve_ips(host: &str) -> Vec<IpAddr> {
    let host = host.to_string();
    let lookup = tokio::net::lookup_host((host.as_str(), 443u16));

    let Ok(Ok(addrs)) = tokio::time::timeout(PROBE_TIMEOUT, lookup).await else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    addrs
        .map(|addr| addr.ip())
        .filter(|ip| seen.insert(*ip))
        .collect()
}

async fn check_tls(domain: &str) -> TlsHealth {
    let unknown = TlsHealth {
        status: TlsStatus::Unknown,
        issuer: None,
        expires_at: None,
        days_until_expiry: None,
    };

    let client = match reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .tls_info(true)
        .timeout(PROBE_TIMEOUT)
        .build()
    {
        Ok(client) => client,
        Err(_) => return unknown,
    };

    let response = match client.get(format!("https://{domain}/")).send().await {
        Ok(response) => response,
        Err(_) => {
            return TlsHealth {
                status: TlsStatus::Unreachable,
                ..unknown
            };
        }
    };

    let Some(der) = response
        .extensions()
        .get::<TlsInfo>()
        .and_then(|info| info.peer_certificate())
    else {
        return unknown;
    };

    inspect_certificate(der, domain).unwrap_or(unknown)
}

fn inspect_certificate(der: &[u8], domain: &str) -> Option<TlsHealth> {
    let (_, cert) = X509Certificate::from_der(der).ok()?;

    let issuer = cert
        .issuer()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .map(|cn| cn.to_string());

    let not_after = cert.validity().not_after.timestamp();
    let now = chrono::Utc::now().timestamp();
    let days_until_expiry = (not_after - now).div_euclid(86_400);

    let expires_at = chrono::DateTime::from_timestamp(not_after, 0).map(|dt| dt.to_rfc3339());

    let status = if now > not_after {
        TlsStatus::Expired
    } else if certificate_covers(&cert, domain) {
        TlsStatus::Active
    } else {
        TlsStatus::Pending
    };

    Some(TlsHealth {
        status,
        issuer,
        expires_at,
        days_until_expiry: Some(days_until_expiry),
    })
}

fn certificate_covers(cert: &X509Certificate, domain: &str) -> bool {
    let Ok(Some(san)) = cert.subject_alternative_name() else {
        return false;
    };

    san.value.general_names.iter().any(|name| match name {
        GeneralName::DNSName(entry) => host_matches(entry, domain),
        _ => false,
    })
}

fn host_matches(pattern: &str, domain: &str) -> bool {
    if pattern.eq_ignore_ascii_case(domain) {
        return true;
    }

    let Some(suffix) = pattern.strip_prefix("*.") else {
        return false;
    };

    match domain.split_once('.') {
        Some((_, rest)) => rest.eq_ignore_ascii_case(suffix),
        None => false,
    }
}
