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
        _ => {}
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

    let status = if resolved.is_empty() {
        DnsStatus::Unresolved
    } else if expected_ips.is_empty() {
        DnsStatus::Unknown
    } else if resolved.iter().any(|ip| expected_set.contains(ip)) {
        DnsStatus::Ok
    } else {
        DnsStatus::Mismatch
    };

    DnsHealth {
        status,
        resolved_ips: resolved.iter().map(|ip| ip.to_string()).collect(),
        expected_ips: expected_ips.iter().map(|ip| ip.to_string()).collect(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_case_insensitive_match() {
        assert!(host_matches("app.example.com", "app.example.com"));
        assert!(host_matches("App.Example.com", "app.example.com"));
        assert!(!host_matches("other.example.com", "app.example.com"));
    }

    #[test]
    fn wildcard_matches_single_label_only() {
        assert!(host_matches("*.example.com", "app.example.com"));
        assert!(!host_matches("*.example.com", "example.com"));
        assert!(!host_matches("*.example.com", "a.b.example.com"));
    }

    #[test]
    fn dns_failure_overrides_a_serving_certificate() {
        assert!(matches!(
            overall_status(&DnsStatus::Mismatch, &TlsStatus::Active),
            HealthStatus::Error
        ));
        assert!(matches!(
            overall_status(&DnsStatus::Unresolved, &TlsStatus::Active),
            HealthStatus::Error
        ));
    }

    #[test]
    fn overall_status_follows_tls_when_dns_is_fine() {
        assert!(matches!(
            overall_status(&DnsStatus::Ok, &TlsStatus::Active),
            HealthStatus::Healthy
        ));
        assert!(matches!(
            overall_status(&DnsStatus::Ok, &TlsStatus::Pending),
            HealthStatus::Pending
        ));
        assert!(matches!(
            overall_status(&DnsStatus::Unknown, &TlsStatus::Active),
            HealthStatus::Healthy
        ));
    }
}
