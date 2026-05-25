// apps\web\src\api
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
//! # 重连退避策略
//!
//! 本模块提供 WebSocket 重连的指数退避策略。
//! 遵循 `05_network.md` 规范: 1s → 2s → 4s → 8s → 16s → 30s (cap), 含 Jitter。

use gloo_timers::future::TimeoutFuture;

/// 连接重试的指数退避策略 (Exponential Backoff with Jitter)
pub struct BackoffStrategy {
    current_ms: u32,
}

impl BackoffStrategy {
    const INITIAL_MS: u32 = 1000;
    const MAX_MS: u32 = 30000;

    pub fn new() -> Self {
        Self {
            current_ms: Self::INITIAL_MS,
        }
    }

    pub fn reset(&mut self) {
        self.current_ms = Self::INITIAL_MS;
    }

    pub async fn wait(&mut self) {
        // Jitter: ±25% 随机偏移，避免多客户端同时重连 (thundering herd)
        let jitter = (self.current_ms / 4) as i32;
        let offset = (js_sys::Math::random() * (2.0 * jitter as f64)) as i32 - jitter;
        let delay = (self.current_ms as i32 + offset).max(100) as u32;
        leptos::logging::log!("WS: Reconnecting in {}ms...", delay);
        TimeoutFuture::new(delay).await;
        self.current_ms = (self.current_ms * 2).min(Self::MAX_MS);
    }
}
