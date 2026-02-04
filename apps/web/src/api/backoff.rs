// apps\web\src\api
//! # 重连退避策略
//!
//! 本模块提供 WebSocket 重连的指数退避策略（1秒起始，最大10秒）。

use gloo_timers::future::TimeoutFuture;

/// 连接重试的指数退避策略
pub struct BackoffStrategy {
    current_ms: u32,
}

impl BackoffStrategy {
    const INITIAL_MS: u32 = 1000;
    const MAX_MS: u32 = 10000;

    pub fn new() -> Self {
        Self {
            current_ms: Self::INITIAL_MS,
        }
    }

    pub fn reset(&mut self) {
        self.current_ms = Self::INITIAL_MS;
    }

    pub async fn wait(&mut self) {
        leptos::logging::log!("WS: Reconnecting in {}ms...", self.current_ms);
        TimeoutFuture::new(self.current_ms).await;
        self.current_ms = (self.current_ms * 2).min(Self::MAX_MS);
    }
}
