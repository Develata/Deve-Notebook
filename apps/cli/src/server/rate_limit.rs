// apps/cli/src/server/rate_limit.rs
//! # Per-IP 速率限制中间件
//!
//! **架构作用**:
//! 防止单个 IP 过度请求 HTTP API 或 WebSocket 升级。
//! 使用滑动窗口计数器实现，零外部依赖。
//!
//! **Invariants**:
//! - 每个 IP 在 `window` 时间内最多 `max_requests` 次请求
//! - 过期条目在每次检查时惰性清除
//! - 状态使用 `Arc<Mutex<...>>` 跨请求共享，锁持有时间 < 1μs
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
    timestamps: Vec<Instant>,
}

/// 速率限制器 (线程安全、可 Clone)
///
/// # Pre-conditions
/// - `max_requests > 0`
/// - `window > Duration::ZERO`
#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<HashMap<IpAddr, IpRecord>>>,
    max_requests: u32,
    window: Duration,
}

/// GC 触发阈值: 当 IP 数量超过此值时执行全局清理
const GC_THRESHOLD: usize = 1024;

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    /// 检查 IP 是否允许请求。允许则返回 `true`。
    ///
    /// # Post-conditions
    /// - 过期时间戳已被清除
    /// - 若允许，当前时间戳已记录
    fn check_and_record(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window;

        let Ok(mut map) = self.state.lock() else {
            return true; // Mutex 中毒时放行 (fail-open)
        };

        // 惰性 GC: IP 数量超阈值时清理所有过期条目
        if map.len() > GC_THRESHOLD {
            map.retain(|_, r| {
                r.timestamps.retain(|t| *t > cutoff);
                !r.timestamps.is_empty()
            });
        }

        let record = map.entry(ip).or_insert_with(|| IpRecord {
            timestamps: Vec::new(),
        });

        record.timestamps.retain(|t| *t > cutoff);

        if record.timestamps.len() >= self.max_requests as usize {
            return false;
        }

        record.timestamps.push(now);
        true
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
    if !limiter.check_and_record(addr.ip()) {
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

        assert!(limiter.check_and_record(ip));
        assert!(limiter.check_and_record(ip));
        assert!(limiter.check_and_record(ip));
    }

    #[test]
    fn test_rejects_over_limit() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(limiter.check_and_record(ip));
        assert!(limiter.check_and_record(ip));
        assert!(!limiter.check_and_record(ip)); // 第 3 次被拒
    }

    #[test]
    fn test_different_ips_independent() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let ip_a: IpAddr = "10.0.0.1".parse().unwrap();
        let ip_b: IpAddr = "10.0.0.2".parse().unwrap();

        assert!(limiter.check_and_record(ip_a));
        assert!(!limiter.check_and_record(ip_a));
        assert!(limiter.check_and_record(ip_b)); // 不同 IP 独立计数
    }

    #[test]
    fn test_window_expiration() {
        let limiter = RateLimiter::new(1, Duration::from_millis(50));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(limiter.check_and_record(ip));
        assert!(!limiter.check_and_record(ip));

        std::thread::sleep(Duration::from_millis(60));
        assert!(limiter.check_and_record(ip)); // 窗口过期后恢复
    }
}
