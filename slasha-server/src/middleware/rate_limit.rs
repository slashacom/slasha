use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;

#[derive(Clone, Copy)]
pub struct RateLimit {
    pub max_requests: u32,
    pub window: Duration,
}

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<RateLimiterInner>,
}

struct RateLimiterInner {
    limit: RateLimit,
    buckets: DashMap<IpAddr, Bucket>,
}

struct Bucket {
    count: u32,
    window_start: Instant,
}

impl RateLimiter {
    pub fn new(limit: RateLimit) -> Self {
        Self {
            inner: Arc::new(RateLimiterInner {
                limit,
                buckets: DashMap::new(),
            }),
        }
    }

    fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = self.inner.limit.window;
        let max = self.inner.limit.max_requests;

        let mut entry = self.inner.buckets.entry(ip).or_insert(Bucket {
            count: 0,
            window_start: now,
        });

        if now.duration_since(entry.window_start) > window {
            entry.count = 0;
            entry.window_start = now;
        }

        if entry.count >= max {
            return false;
        }
        entry.count += 1;
        true
    }
}

pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let ip = client_ip(request.headers());

    if let Some(ip) = ip
        && !limiter.check(ip)
    {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, "60")],
            "rate limit exceeded\n",
        )
            .into_response();
    }

    next.run(request).await
}

fn client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .and_then(|s| s.parse().ok())
}
