//! # 静态文件服务模块
//!
//! 为生产部署提供 SPA 静态文件托管。
//! `trunk build --release` 的产物（WASM + JS + CSS）由此模块通过
//! `tower_http::services::ServeDir` 提供服务。
//!
//! ## 环境变量
//!
//! - `DEVE_STATIC_DIR`: 静态文件目录路径（默认 `./static`）
//!
//! ## 不变量 (Invariants)
//!
//! - 所有非 API / 非 WS 路由 fallback 到 `index.html`（SPA 路由）
//! - API 路由 (`/api/*`, `/ws`) 优先级高于静态文件

use axum::Router;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};

/// 环境变量名: 静态文件目录
const ENV_STATIC_DIR: &str = "DEVE_STATIC_DIR";

/// 默认静态文件目录 (相对于工作目录)
const DEFAULT_STATIC_DIR: &str = "./static";

/// 获取静态文件目录路径
///
/// 优先读取 `DEVE_STATIC_DIR` 环境变量，回退到默认值 `./static`。
fn resolve_static_dir() -> PathBuf {
    std::env::var(ENV_STATIC_DIR)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_STATIC_DIR))
}

/// 构建 SPA 静态文件 fallback 服务
///
/// ## 前置条件
/// - `static_dir` 目录存在且包含 `index.html`
///
/// ## 后置条件
/// - 返回的 Router 将所有未匹配路由 fallback 到静态文件
/// - 不存在的路径返回 `index.html`（SPA 客户端路由）
pub fn static_fallback<S: Clone + Send + Sync + 'static>() -> Router<S> {
    let dir = resolve_static_dir();

    if !dir.exists() {
        tracing::warn!("Static dir {:?} not found — SPA fallback disabled", dir);
        return Router::new();
    }

    let index = dir.join("index.html");
    if !index.exists() {
        tracing::warn!("index.html not found in {:?} — SPA fallback disabled", dir);
        return Router::new();
    }

    tracing::info!("Serving static files from {:?}", dir);

    // ServeDir + fallback 到 index.html 实现 SPA 路由
    let serve = ServeDir::new(&dir).not_found_service(ServeFile::new(index));

    Router::new().fallback_service(serve)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_static_dir() {
        // 确保默认路径不 panic
        unsafe { std::env::remove_var(ENV_STATIC_DIR) };
        let dir = resolve_static_dir();
        assert_eq!(dir, PathBuf::from(DEFAULT_STATIC_DIR));
    }

    #[test]
    fn test_env_override() {
        unsafe { std::env::set_var(ENV_STATIC_DIR, "/custom/path") };
        let dir = resolve_static_dir();
        assert_eq!(dir, PathBuf::from("/custom/path"));
        unsafe { std::env::remove_var(ENV_STATIC_DIR) };
    }
}
