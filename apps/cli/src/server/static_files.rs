//! plan_ref:
//!   - 08_ui_design_01_web#single-binary-distribution
//!
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

use anyhow::{Context, Result, anyhow};
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

/// 校验显式配置的静态目录。
///
/// Invariants:
/// - 显式 `DEVE_STATIC_DIR` 代表操作者声明需要托管静态资源。
/// - 显式配置损坏时必须 fail-closed，不能静默降级成 API-only。
pub fn validate_static_dir_override() -> Result<()> {
    let Ok(raw_dir) = std::env::var(ENV_STATIC_DIR) else {
        return Ok(());
    };
    let dir = PathBuf::from(raw_dir);
    validate_static_root(&dir)
}

fn validate_static_root(dir: &std::path::Path) -> Result<()> {
    match dir.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            return Err(anyhow!("Configured static dir missing: {:?}", dir));
        }
        Err(err) => {
            return Err(err).with_context(|| format!("Failed to stat static dir {:?}", dir));
        }
    }

    let index = dir.join("index.html");
    match index.try_exists() {
        Ok(true) => Ok(()),
        Ok(false) => Err(anyhow!("Configured static index missing: {:?}", index)),
        Err(err) => Err(err).with_context(|| format!("Failed to stat index.html {:?}", index)),
    }
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

    match dir.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            tracing::warn!(
                "Static dir {:?} not found — SPA fallback disabled. Local UI dev: run \
                 `NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080` from apps/web; \
                 production: build frontend and set DEVE_STATIC_DIR.",
                dir
            );
            return Router::new();
        }
        Err(err) => {
            tracing::warn!("Failed to stat static dir {:?}: {}", dir, err);
            return Router::new();
        }
    }

    let index = dir.join("index.html");
    match index.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            tracing::warn!("index.html not found in {:?} — SPA fallback disabled", dir);
            return Router::new();
        }
        Err(err) => {
            tracing::warn!("Failed to stat index.html {:?}: {}", index, err);
            return Router::new();
        }
    }

    tracing::info!("Serving static files from {:?}", dir);

    // ServeDir + fallback 到 index.html 实现 SPA 路由
    let serve = ServeDir::new(&dir).not_found_service(ServeFile::new(index));

    Router::new().fallback_service(serve)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    #[test]
    fn test_default_static_dir() {
        let _guard = env_guard();
        // 确保默认路径不 panic
        unsafe { std::env::remove_var(ENV_STATIC_DIR) };
        let dir = resolve_static_dir();
        assert_eq!(dir, PathBuf::from(DEFAULT_STATIC_DIR));
    }

    #[test]
    fn test_env_override() {
        let _guard = env_guard();
        unsafe { std::env::set_var(ENV_STATIC_DIR, "/custom/path") };
        let dir = resolve_static_dir();
        assert_eq!(dir, PathBuf::from("/custom/path"));
        unsafe { std::env::remove_var(ENV_STATIC_DIR) };
    }

    #[test]
    fn validate_static_dir_override_fails_closed_when_dir_missing() {
        let _guard = env_guard();
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("missing-static");
        unsafe { std::env::set_var(ENV_STATIC_DIR, &missing) };

        let err = validate_static_dir_override()
            .expect_err("configured missing static dir must fail closed");

        unsafe { std::env::remove_var(ENV_STATIC_DIR) };
        assert!(err.to_string().contains("Configured static dir missing"));
    }

    #[test]
    fn validate_static_dir_override_fails_closed_when_index_missing() {
        let _guard = env_guard();
        let dir = tempfile::tempdir().expect("tempdir");
        let static_dir = dir.path().join("static");
        std::fs::create_dir_all(&static_dir).expect("mkdir static");
        unsafe { std::env::set_var(ENV_STATIC_DIR, &static_dir) };

        let err = validate_static_dir_override()
            .expect_err("configured static dir without index must fail closed");

        unsafe { std::env::remove_var(ENV_STATIC_DIR) };
        assert!(
            err.to_string().contains("Configured static index missing")
                || err.to_string().contains("index.html")
        );
    }
}
