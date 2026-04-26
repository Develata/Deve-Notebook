// apps/cli/src/server/router.rs
//! # 路由构建模块
//!
//! 负责 Axum Router 的组装：认证路由、公开路由、中间件层叠。

use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

use super::{AppState, auth, handlers, node_role_http, rate_limit, setup, static_files, ws};
use deve_core::security::AuthConfig;
/// 构建完整的 Axum 应用路由
///
/// ## 路由结构
/// - **protected**: 需要 JWT Cookie 认证的路由 (`/ws`, `/api/sc/*`, `/api/repo/*`, `/api/auth/me`)
/// - **public**: 无需认证的路由 (`/api/auth/login`, `/api/auth/status`, `/api/node/role`)
/// ## 中间件层叠顺序（外 → 内）
/// CORS → 速率限制 → 安全头 → Extension 注入
pub fn build_app(
    app_state: Arc<AppState>,
    port: u16,
    auth_config: Arc<AuthConfig>,
) -> Result<Router> {
    let brute_force = Arc::new(auth::brute_force::BruteForceGuard::new());
    let login_limiter = rate_limit::RateLimiter::new(5, std::time::Duration::from_secs(60));
    let api_limiter = rate_limit::RateLimiter::new(120, std::time::Duration::from_secs(60));

    let protected = Router::new()
        .route("/ws", get(ws::ws_handler))
        .route(
            "/api/sc/pending",
            get(handlers::source_control::http::pending),
        )
        .route(
            "/api/sc/status",
            get(handlers::source_control::http::status),
        )
        .route("/api/sc/diff", get(handlers::source_control::http::diff))
        .route(
            "/api/sc/commits",
            get(handlers::source_control::http_commits::commit_history),
        )
        .route(
            "/api/sc/commit-diff",
            get(handlers::source_control::http_commits::commit_diff),
        )
        .route(
            "/api/sc/stage-pending",
            post(handlers::source_control::http_mutations::stage),
        )
        .route(
            "/api/sc/unstage",
            post(handlers::source_control::http_mutations::unstage),
        )
        .route(
            "/api/sc/discard-pending",
            post(handlers::source_control::http_mutations::discard_pending),
        )
        .route(
            "/api/sc/commit",
            post(handlers::source_control::http_mutations::commit),
        )
        .route("/api/repo/docs", get(handlers::repo::http::list_docs))
        .route("/api/repo/doc", get(handlers::repo::http::doc_content))
        .route("/api/auth/logout", post(auth::handlers::logout))
        .route("/api/auth/me", get(auth::handlers::me))
        .route("/api/admin/dump", get(handlers::admin::dump))
        .route("/api/admin/export", get(handlers::admin::export))
        .route("/api/admin/node-check", get(handlers::admin::node_check))
        .route(
            "/api/admin/projection-check",
            get(handlers::admin::projection_check),
        )
        .layer(axum::middleware::from_fn(auth::middleware::auth_middleware));

    let login_route = Router::new()
        .route("/api/auth/login", post(auth::handlers::login))
        .layer(axum::Extension(login_limiter))
        .layer(axum::middleware::from_fn(
            auth::middleware::login_rate_limit_middleware,
        ));

    let public = Router::new()
        .route("/api/node/role", get(node_role_http::role))
        .route("/api/auth/status", get(auth::handlers::status))
        .route(
            "/api/ai/backend-capabilities",
            get(super::agent_bridge::http_backend_capabilities),
        );

    Ok(Router::new()
        .merge(protected)
        .merge(public)
        .merge(login_route)
        .merge(static_files::static_fallback())
        .with_state(app_state)
        .layer(axum::middleware::from_fn(auth::headers::security_headers))
        .layer(axum::middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(axum::Extension(auth_config))
        .layer(axum::Extension(brute_force))
        .layer(axum::Extension(api_limiter))
        .layer(setup::build_cors_layer(port)?))
}

/// 按环境驱动契约加载认证配置；生产缺失密钥时直接以非零退出失败。
/// 开发模式必须显式设置 DEVE_ENV=development（不再自动根据构建模式切换）。
pub(super) fn load_auth_config() -> AuthConfig {
    let is_dev_mode =
        matches!(std::env::var("DEVE_ENV"), Ok(value) if value.eq_ignore_ascii_case("development"));
    let using_dev_defaults = is_dev_mode
        && (std::env::var("AUTH_SECRET").is_err() || std::env::var("AUTH_PASS").is_err());

    match AuthConfig::from_env() {
        Ok(cfg) => {
            if using_dev_defaults {
                tracing::warn!(
                    "WARNING: Development mode with default credentials (set AUTH_SECRET and AUTH_PASS for production)"
                );
            } else {
                tracing::info!("Auth config loaded from env (user={})", cfg.username);
            }
            cfg
        }
        Err(err) => {
            tracing::error!(
                "{err} (local dev: use `deve_cli serve --dev` or set DEVE_ENV=development)"
            );
            std::process::exit(1);
        }
    }
}
