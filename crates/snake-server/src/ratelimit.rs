//! A small fixed-window per-IP rate limiter.
//!
//! Deliberately dependency-free and in-memory: the leaderboard server is a
//! single hobby instance, so a `HashMap<IpAddr, window>` behind a mutex is
//! plenty. Stale entries are pruned opportunistically so the map cannot grow
//! without bound under a stream of distinct IPs.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct RateLimiter {
    window_secs: u64,
    max: u32,
    hits: HashMap<IpAddr, (u64, u32)>,
}

impl RateLimiter {
    pub fn per_minute(max: u32) -> Self {
        Self {
            window_secs: 60,
            max,
            hits: HashMap::new(),
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Record a hit from `ip`; returns `true` if it is within the limit.
    /// `max == 0` disables limiting (always allowed).
    pub fn check(&mut self, ip: IpAddr) -> bool {
        if self.max == 0 {
            return true;
        }
        let now = Self::now();
        if self.hits.len() > 10_000 {
            let cutoff = now.saturating_sub(self.window_secs);
            self.hits.retain(|_, (start, _)| *start >= cutoff);
        }
        let slot = self.hits.entry(ip).or_insert((now, 0));
        if now.saturating_sub(slot.0) >= self.window_secs {
            *slot = (now, 0);
        }
        slot.1 += 1;
        slot.1 <= self.max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_limit_then_blocks() {
        let mut rl = RateLimiter::per_minute(3);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(rl.check(ip));
        assert!(rl.check(ip));
        assert!(rl.check(ip));
        assert!(!rl.check(ip), "fourth hit in the window is blocked");
    }

    #[test]
    fn separate_ips_have_separate_budgets() {
        let mut rl = RateLimiter::per_minute(1);
        let a: IpAddr = "10.0.0.1".parse().unwrap();
        let b: IpAddr = "10.0.0.2".parse().unwrap();
        assert!(rl.check(a));
        assert!(rl.check(b), "different IP is independent");
        assert!(!rl.check(a));
    }

    #[test]
    fn zero_disables_limiting() {
        let mut rl = RateLimiter::per_minute(0);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        for _ in 0..1000 {
            assert!(rl.check(ip));
        }
    }
}
