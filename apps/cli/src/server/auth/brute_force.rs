//! plan_ref:
//!   - 08_auth#auth-rate-limiting
//!
//! # 暴力破解防护 (Brute Force Protection)
//!
//! 09_auth.md: "连续 5 次登录失败后 IP 封禁 15 分钟"
//!
//! ## Invariants
//! - 每个 IP 独立计数
//! - 封禁窗口从最后一次失败开始计算
//! - 登录成功后立即清除该 IP 的失败记录
//! - live-IP 状态有硬上限；容量耗尽时未登记 IP fail-closed
//! - 热路径只清理当前 IP；全局过期清理只在新 IP 遭遇容量压力时执行

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 最大连续失败次数
const MAX_FAILURES: u32 = 5;
/// 封禁持续时间
const BAN_DURATION: Duration = Duration::from_secs(15 * 60);
/// 生产 guard 最多保留的 live IP 数。
const MAX_TRACKED_IPS: usize = 4096;

struct IpRecord {
    failures: u32,
    expires_at: Instant,
}

struct BruteForceState {
    records: HashMap<IpAddr, IpRecord>,
    capacity_warning_emitted: bool,
}

pub struct BruteForceGuard {
    state: Mutex<BruteForceState>,
    max_tracked_ips: usize,
}

impl Default for BruteForceGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl BruteForceGuard {
    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, BruteForceState>, ()> {
        self.state.lock().map_err(|_| {
            tracing::error!("BruteForceGuard lock poisoned; failing closed");
        })
    }

    pub fn new() -> Self {
        Self::with_capacity(MAX_TRACKED_IPS)
    }

    fn with_capacity(max_tracked_ips: usize) -> Self {
        assert!(
            max_tracked_ips > 0,
            "brute force tracked-IP capacity must be positive"
        );
        Self {
            state: Mutex::new(BruteForceState {
                records: HashMap::new(),
                capacity_warning_emitted: false,
            }),
            max_tracked_ips,
        }
    }

    /// 检查 IP 是否被封禁
    pub fn is_blocked(&self, ip: &IpAddr) -> bool {
        let now = Instant::now();
        let Ok(mut state) = self.lock_state() else {
            return true;
        };
        if let Some(record) = state.records.get(ip) {
            if record.expires_at > now {
                return record.failures >= MAX_FAILURES;
            }
            state.records.remove(ip);
            state.capacity_warning_emitted = false;
            return false;
        }
        if state.records.len() >= self.max_tracked_ips {
            state.reclaim_expired_capacity(now);
            if state.records.len() < self.max_tracked_ips {
                state.capacity_warning_emitted = false;
            }
        }
        if state.records.len() >= self.max_tracked_ips {
            if !state.capacity_warning_emitted {
                tracing::warn!(
                    tracked_ips = state.records.len(),
                    capacity = self.max_tracked_ips,
                    "BruteForceGuard live-IP capacity exhausted; blocking new IP"
                );
                state.capacity_warning_emitted = true;
            }
            return true;
        }
        false
    }

    /// 记录一次登录失败
    pub fn record_failure(&self, ip: &IpAddr) {
        let now = Instant::now();
        let expires_at = now + BAN_DURATION;
        let Ok(mut state) = self.lock_state() else {
            return;
        };
        if let Some(entry) = state.records.get_mut(ip) {
            if entry.expires_at <= now {
                entry.failures = 1;
            } else {
                entry.failures = entry.failures.saturating_add(1);
            }
            entry.expires_at = expires_at;
            return;
        }
        if state.records.len() >= self.max_tracked_ips {
            state.reclaim_expired_capacity(now);
            if state.records.len() < self.max_tracked_ips {
                state.capacity_warning_emitted = false;
            }
        }
        if state.records.len() >= self.max_tracked_ips {
            if !state.capacity_warning_emitted {
                tracing::warn!(
                    tracked_ips = state.records.len(),
                    capacity = self.max_tracked_ips,
                    "BruteForceGuard live-IP capacity exhausted; refusing untracked failure record"
                );
                state.capacity_warning_emitted = true;
            }
            return;
        }
        state.records.insert(
            *ip,
            IpRecord {
                failures: 1,
                expires_at,
            },
        );
    }

    /// 登录成功后清除记录
    pub fn record_success(&self, ip: &IpAddr) {
        let Ok(mut state) = self.lock_state() else {
            return;
        };
        if state.records.remove(ip).is_some() {
            state.capacity_warning_emitted = false;
        }
    }
}

impl BruteForceState {
    fn reclaim_expired_capacity(&mut self, now: Instant) {
        self.records.retain(|_, record| record.expires_at > now);
    }
}

#[cfg(test)]
mod tests;
