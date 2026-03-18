// apps/cli/src/server/auth/brute_force.rs
//! # 暴力破解防护 (Brute Force Protection)
//!
//! 09_auth.md: "连续 5 次登录失败后 IP 封禁 15 分钟"
//!
//! ## Invariants
//! - 每个 IP 独立计数
//! - 封禁窗口从最后一次失败开始计算
//! - 登录成功后立即清除该 IP 的失败记录
//! - 过期条目惰性清除，防止内存无限增长

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 最大连续失败次数
const MAX_FAILURES: u32 = 5;
/// 封禁持续时间
const BAN_DURATION: Duration = Duration::from_secs(15 * 60);
/// 惰性 GC 阈值
const GC_THRESHOLD: usize = 512;

struct IpRecord {
    failures: u32,
    last_failure: Instant,
}

pub struct BruteForceGuard {
    records: Mutex<HashMap<IpAddr, IpRecord>>,
}

impl BruteForceGuard {
    fn lock_records(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<IpAddr, IpRecord>>, ()> {
        self.records.lock().map_err(|_| {
            tracing::error!("BruteForceGuard lock poisoned; failing closed");
        })
    }

    pub fn new() -> Self {
        Self {
            records: Mutex::new(HashMap::new()),
        }
    }

    /// 检查 IP 是否被封禁
    pub fn is_blocked(&self, ip: &IpAddr) -> bool {
        let Ok(records) = self.lock_records() else {
            return true;
        };
        match records.get(ip) {
            Some(r) if r.failures >= MAX_FAILURES => r.last_failure.elapsed() < BAN_DURATION,
            _ => false,
        }
    }

    /// 记录一次登录失败
    pub fn record_failure(&self, ip: &IpAddr) {
        let Ok(mut records) = self.lock_records() else {
            return;
        };
        let entry = records.entry(*ip).or_insert(IpRecord {
            failures: 0,
            last_failure: Instant::now(),
        });
        entry.failures += 1;
        entry.last_failure = Instant::now();

        // 惰性 GC
        if records.len() > GC_THRESHOLD {
            records.retain(|_, r| r.last_failure.elapsed() < BAN_DURATION);
        }
    }

    /// 登录成功后清除记录
    pub fn record_success(&self, ip: &IpAddr) {
        let Ok(mut records) = self.lock_records() else {
            return;
        };
        records.remove(ip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_blocked_initially() {
        let guard = BruteForceGuard::new();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(!guard.is_blocked(&ip));
    }

    #[test]
    fn test_blocked_after_max_failures() {
        let guard = BruteForceGuard::new();
        let ip: IpAddr = "10.0.0.2".parse().unwrap();
        for _ in 0..MAX_FAILURES {
            guard.record_failure(&ip);
        }
        assert!(guard.is_blocked(&ip));
    }

    #[test]
    fn test_cleared_on_success() {
        let guard = BruteForceGuard::new();
        let ip: IpAddr = "10.0.0.3".parse().unwrap();
        for _ in 0..MAX_FAILURES {
            guard.record_failure(&ip);
        }
        assert!(guard.is_blocked(&ip));
        guard.record_success(&ip);
        assert!(!guard.is_blocked(&ip));
    }

    #[test]
    fn test_four_failures_not_blocked() {
        let guard = BruteForceGuard::new();
        let ip: IpAddr = "10.0.0.4".parse().unwrap();
        for _ in 0..MAX_FAILURES - 1 {
            guard.record_failure(&ip);
        }
        assert!(!guard.is_blocked(&ip));
    }

    #[test]
    fn poisoned_lock_blocks_ip_fail_closed() {
        let guard = BruteForceGuard::new();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = guard.records.lock().expect("lock");
            panic!("poison brute force guard");
        }));
        let ip: IpAddr = "10.0.0.5".parse().unwrap();
        assert!(guard.is_blocked(&ip));
    }
}
