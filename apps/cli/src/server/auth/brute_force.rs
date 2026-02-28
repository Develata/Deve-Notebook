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
    pub fn new() -> Self {
        Self {
            records: Mutex::new(HashMap::new()),
        }
    }

    /// 检查 IP 是否被封禁
    pub fn is_blocked(&self, ip: &IpAddr) -> bool {
        let records = self.records.lock().unwrap_or_else(|e| e.into_inner());
        match records.get(ip) {
            Some(r) if r.failures >= MAX_FAILURES => {
                r.last_failure.elapsed() < BAN_DURATION
            }
            _ => false,
        }
    }

    /// 记录一次登录失败
    pub fn record_failure(&self, ip: &IpAddr) {
        let mut records = self.records.lock().unwrap_or_else(|e| e.into_inner());
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
        let mut records = self.records.lock().unwrap_or_else(|e| e.into_inner());
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
}
