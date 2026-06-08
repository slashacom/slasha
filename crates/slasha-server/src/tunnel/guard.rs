use dashmap::DashMap;
use once_cell::sync::Lazy;

use super::TunnelError;

static USER_TUNNEL_COUNTS: Lazy<DashMap<String, usize>> = Lazy::new(DashMap::new);

pub const MAX_TUNNELS_PER_USER: usize = 10;

pub struct TunnelGuard {
    user_id: String,
}

impl TunnelGuard {
    pub fn try_acquire(user_id: &str) -> Result<Self, TunnelError> {
        let mut entry = USER_TUNNEL_COUNTS.entry(user_id.to_string()).or_insert(0);
        if *entry >= MAX_TUNNELS_PER_USER {
            return Err(TunnelError::LimitReached(MAX_TUNNELS_PER_USER));
        }
        *entry += 1;
        Ok(Self {
            user_id: user_id.to_string(),
        })
    }
}

impl Drop for TunnelGuard {
    fn drop(&mut self) {
        if let Some(mut entry) = USER_TUNNEL_COUNTS.get_mut(&self.user_id) {
            *entry = entry.saturating_sub(1);
        }
    }
}
