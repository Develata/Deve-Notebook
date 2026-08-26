// apps/cli/src/server/rate_limit.rs
//! plan_ref:
//!   - 08_auth#auth-rate-limiting
//!
//! # Per-IP 速率限制中间件
//!
//! **架构作用**:
//! 防止单个 IP 过度请求 HTTP API 或 WebSocket 升级。
//! 使用滑动窗口计数器实现，零外部依赖。
//!
//! **Invariants**:
//! - 每个 IP 在 `window` 时间内最多 `max_requests` 次请求
//! - live-IP 状态有硬上限；容量耗尽时拒绝新 IP，不驱逐仍有效记录
//! - 热路径只清理当前 IP；全局过期清理只在新 IP 遭遇容量压力时执行
//! - 状态使用 `Arc<Mutex<...>>` 跨请求共享，锁内只执行有界内存操作，不做 I/O
//!
//! **集成方式**:
//! ```ignore
//! let limiter = RateLimiter::new(100, Duration::from_secs(60));
//! let app = Router::new()
//!     .layer(Extension(limiter))
//!     .layer(axum::middleware::from_fn(rate_limit_middleware));
//! axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
//! ```

use axum::{
    Extension,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 单个 IP 的滑动窗口请求记录
struct IpRecord {
    expirations: Vec<Instant>,
}

struct RateLimitState {
    records: HashMap<IpAddr, IpRecord>,
    capacity_warning_emitted: bool,
}

/// 速率限制器 (线程安全、可 Clone)
///
/// # Pre-conditions
/// - `max_requests > 0`
/// - `window > Duration::ZERO`
#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<RateLimitState>>,
    max_requests: usize,
    max_tracked_ips: usize,
    window: Duration,
}

/// 同一个 limiter 最多保留的 live IP 数。两个生产 limiter 各自独立受限。
const MAX_TRACKED_IPS: usize = 4096;

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self::with_capacity(max_requests, window, MAX_TRACKED_IPS)
    }

    fn with_capacity(max_requests: u32, window: Duration, max_tracked_ips: usize) -> Self {
        assert!(max_requests > 0, "rate limit max_requests must be positive");
        assert!(!window.is_zero(), "rate limit window must be positive");
        assert!(
            max_tracked_ips > 0,
            "rate limit tracked-IP capacity must be positive"
        );
        Self {
            state: Arc::new(Mutex::new(RateLimitState {
                records: HashMap::new(),
                capacity_warning_emitted: false,
            })),
            max_requests: max_requests as usize,
            max_tracked_ips,
            window,
        }
    }

    /// 检查 IP 是否允许请求。允许则返回 `true`。
    ///
    /// # Post-conditions
    /// - 过期时间戳已被清除
    /// - 若允许，当前时间戳已记录
    pub fn check_and_record_ip(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let Some(expires_at) = now.checked_add(self.window) else {
            tracing::error!("RateLimiter window cannot be represented; failing closed");
            return false;
        };

        let Ok(mut state) = self.state.lock() else {
            tracing::error!("RateLimiter state lock poisoned; failing closed");
            return false;
        };

        if let Some(record) = state.records.get_mut(&ip) {
            record.expirations.retain(|expiry| *expiry > now);
            if record.expirations.len() >= self.max_requests {
                return false;
            }
            record.expirations.push(expires_at);
            return true;
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
                    "RateLimiter live-IP capacity exhausted; rejecting new IP"
                );
                state.capacity_warning_emitted = true;
            }
            return false;
        }

        state.records.insert(
            ip,
            IpRecord {
                expirations: vec![expires_at],
            },
        );
        true
    }

    pub const fn retry_after_secs(&self) -> u64 {
        self.window.as_secs()
    }
}

impl RateLimitState {
    fn reclaim_expired_capacity(&mut self, now: Instant) {
        self.records.retain(|_, record| {
            record.expirations.retain(|expiry| *expiry > now);
            !record.expirations.is_empty()
        });
    }
}

/// Axum 中间件函数: 对每个请求执行 per-IP 速率限制
///
/// 被限流时返回 `429 Too Many Requests` + `Retry-After` 响应头。
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(limiter): Extension<RateLimiter>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if !limiter.check_and_record_ip(addr.ip()) {
        tracing::warn!("Rate limit exceeded for IP: {}", addr.ip());
        let retry_after = limiter.window.as_secs().to_string();
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", retry_after)],
            "Too Many Requests",
        )
            .into_response();
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_within_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        assert!(limiter.check_and_record_ip(ip));
        assert!(limiter.check_and_record_ip(ip));
        assert!(limiter.check_and_record_ip(ip));
    }

    #[test]
    fn test_rejects_over_limit() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(limiter.check_and_record_ip(ip));
        assert!(limiter.check_and_record_ip(ip));
        assert!(!limiter.check_and_record_ip(ip)); // 第 3 次被拒
    }

    #[test]
    fn test_different_ips_independent() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let ip_a: IpAddr = "10.0.0.1".parse().unwrap();
        let ip_b: IpAddr = "10.0.0.2".parse().unwrap();

        assert!(limiter.check_and_record_ip(ip_a));
        assert!(!limiter.check_and_record_ip(ip_a));
        assert!(limiter.check_and_record_ip(ip_b)); // 不同 IP 独立计数
    }

    #[test]
    fn test_window_expiration() {
        let limiter = RateLimiter::new(1, Duration::from_millis(50));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(limiter.check_and_record_ip(ip));
        assert!(!limiter.check_and_record_ip(ip));

        std::thread::sleep(Duration::from_millis(60));
        assert!(limiter.check_and_record_ip(ip)); // 窗口过期后恢复
    }

    #[test]
    fn test_poisoned_state_fails_closed() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let poisoned = limiter.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = poisoned.state.lock().expect("lock");
            panic!("poison rate limiter state");
        }));
        let ip: IpAddr = "10.0.0.9".parse().unwrap();

        assert!(!limiter.check_and_record_ip(ip));
    }

    #[test]
    fn new_ip_is_rejected_when_live_ip_capacity_is_full() {
        let limiter = RateLimiter::with_capacity(2, Duration::from_secs(60), 2);
        let first: IpAddr = "10.0.0.1".parse().unwrap();
        let second: IpAddr = "10.0.0.2".parse().unwrap();
        let overflow: IpAddr = "10.0.0.3".parse().unwrap();

        assert!(limiter.check_and_record_ip(first));
        assert!(limiter.check_and_record_ip(second));
        assert!(!limiter.check_and_record_ip(overflow));
        assert!(limiter.check_and_record_ip(first));
        assert!(!limiter.check_and_record_ip(first));
        let state = limiter.state.lock().expect("state");
        assert_eq!(state.records.len(), 2);
        assert!(state.capacity_warning_emitted);
    }

    #[test]
    fn expired_capacity_is_reclaimed_under_capacity_pressure() {
        let limiter = RateLimiter::with_capacity(1, Duration::from_secs(60), 1);
        let first: IpAddr = "10.0.0.1".parse().unwrap();
        let replacement: IpAddr = "10.0.0.2".parse().unwrap();

        assert!(limiter.check_and_record_ip(first));
        assert!(!limiter.check_and_record_ip(replacement));
        {
            let mut state = limiter.state.lock().expect("state");
            state.records.get_mut(&first).expect("record").expirations = vec![
                Instant::now()
                    .checked_sub(Duration::from_secs(1))
                    .expect("representable expired instant"),
            ];
        }
        assert!(limiter.check_and_record_ip(replacement));
        let state = limiter.state.lock().expect("state");
        assert_eq!(state.records.len(), 1);
        assert!(state.records.contains_key(&replacement));
        assert!(!state.capacity_warning_emitted);
    }
}
