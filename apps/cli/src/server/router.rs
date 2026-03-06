// apps/cli/src/server/router.rs
//! # 路由构建模块
//!
//! 负责 Axum Router 的组装：认证路由、公开路由、中间件层叠。
//! 从 `mod.rs` 拆分以保持单文件行数在 130 行目标以内。

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use super::{auth, handlers, node_role_http, rate_limit, setup, static_files, ws, AppState};

/// 构建完整的 Axum 应用路由
///
/// ## 路由结构
/// - **protected**: 需要 JWT Cookie 认证的路由 (`/ws`, `/api/sc/*`, `/api/repo/*`, `/api/auth/me`)
/// - **public**: 无需认证的路由 (`/api/auth/login`, `/api/node/role`)
/// - **static**: SPA 静态文件回退
///
/// ## 中间件层叠顺序（外 → 内）
/// CORS → 速率限制 → 安全头 → Extension 注入
pub fn build_app(app_state: Arc<AppState>, port: u16) -> Router {
    let auth_config = load_auth_config();
    let auth_config = Arc::new(auth_config);
    let brute_force = Arc::new(auth::brute_force::BruteForceGuard::new());

    // 速率限制: 每 IP 每分钟最多 200 次请求
    let limiter = rate_limit::RateLimiter::new(200, std::time::Duration::from_secs(60));

    // 需要认证的路由 (JWT Cookie 中间件保护)
    let protected = Router::new()
        .route("/ws", get(ws::ws_handler))
        .route(
            "/api/sc/status",
            get(handlers::source_control::http::status),
        )
        .route("/api/sc/diff", get(handlers::source_control::http::diff))
        .route("/api/sc/stage", post(handlers::source_control::http::stage))
        .route(
            "/api/sc/commit",
            post(handlers::source_control::http::commit),
        )
        .route("/api/repo/docs", get(handlers::repo::http::list_docs))
        .route("/api/repo/doc", get(handlers::repo::http::doc_content))
        .route("/api/auth/logout", post(auth::handlers::logout))
        .route("/api/auth/me", get(auth::handlers::me))
        .layer(axum::middleware::from_fn(auth::middleware::auth_middleware));

    // 公开路由 (无需认证)
    let public = Router::new()
        .route("/api/auth/login", post(auth::handlers::login))
        .route("/api/node/role", get(node_role_http::role));

    Router::new()
        .merge(protected)
        .merge(public)
        .merge(static_files::static_fallback())
        .with_state(app_state)
        .layer(axum::middleware::from_fn(auth::headers::security_headers))
        .layer(axum::middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(axum::Extension(auth_config))
        .layer(axum::Extension(brute_force))
        .layer(axum::Extension(limiter))
        .layer(setup::build_cors_layer(port))
}

/// 加载认证配置: 优先环境变量，回退到 dev 默认
fn load_auth_config() -> deve_core::security::AuthConfig {
    match deve_core::security::AuthConfig::from_env() {
        Ok(cfg) => {
            tracing::info!("Auth config loaded from env (user={})", cfg.username);
            cfg
        }
        Err(_) => {
            tracing::warn!("⚠ Auth: env vars not set, using dev defaults (admin/admin)");
            deve_core::security::AuthConfig::dev_default()
                .expect("Dev auth config should always succeed")
        }
    }
}
