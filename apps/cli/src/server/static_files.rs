//! plan_ref:
//!   - 11_ui_design/01_web#single-binary-distribution
//!
//! # 静态文件服务模块
//!
//! 为生产部署提供 SPA 静态文件托管。
//! `trunk build --release` 的产物（WASM + JS + CSS）可通过显式
//! `DEVE_STATIC_DIR` 目录提供，也可由 CLI build script 编译进二进制。
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
use axum::body::Body;
use axum::http::{HeaderValue, Request, Response, StatusCode, header};
use std::convert::Infallible;
use std::path::PathBuf;
use tower::service_fn;
use tower_http::services::ServeDir;

/// 环境变量名: 静态文件目录
const ENV_STATIC_DIR: &str = "DEVE_STATIC_DIR";

/// 默认静态文件目录 (相对于工作目录)
const DEFAULT_STATIC_DIR: &str = "./static";

const TRUNK_DEV_INDEX_MARKERS: &[&[u8]] = &[
    b"{{__TRUNK_ADDRESS__}}",
    b"{{__trunk_address__}}",
    b"{{__TRUNK_WS_BASE__}}",
    b"{{__trunk_ws_base__}}",
    b".well-known/trunk/ws",
];

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

pub fn delivery_shape() -> &'static str {
    let dir = resolve_static_dir();
    classify_delivery_shape(
        validate_static_root(&dir).is_ok(),
        std::env::var_os(ENV_STATIC_DIR).is_some(),
        super::static_files_embed::has_index_asset(),
    )
}

fn classify_delivery_shape(
    static_root_valid: bool,
    static_dir_override: bool,
    embedded_index_valid: bool,
) -> &'static str {
    if static_root_valid {
        if static_dir_override {
            return "static-dir-override";
        }
        return "static-dir";
    }
    if embedded_index_valid {
        return "embedded-frontend";
    }
    "api-only"
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
        Ok(true) => validate_static_index(&index),
        Ok(false) => Err(anyhow!("Configured static index missing: {:?}", index)),
        Err(err) => Err(err).with_context(|| format!("Failed to stat index.html {:?}", index)),
    }
}

fn validate_static_index(index: &std::path::Path) -> Result<()> {
    let bytes = std::fs::read(index)
        .with_context(|| format!("Failed to read static index.html {:?}", index))?;
    if index_html_contains_trunk_dev_ws(&bytes) {
        return Err(anyhow!(
            "Configured static index contains Trunk development live-reload markers: {:?}; run `trunk build --release`, not `trunk serve`",
            index
        ));
    }
    Ok(())
}

pub(super) fn index_html_contains_trunk_dev_ws(bytes: &[u8]) -> bool {
    TRUNK_DEV_INDEX_MARKERS
        .iter()
        .any(|marker| contains_bytes(bytes, marker))
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
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
    static_fallback_from_dir(dir)
}

fn static_fallback_from_dir<S: Clone + Send + Sync + 'static>(dir: PathBuf) -> Router<S> {
    if let Err(err) = validate_static_root(&dir) {
        if let Some(router) = super::static_files_embed::fallback() {
            return router;
        }
        tracing::warn!(
            "Static dir {:?} unavailable ({}) and no embedded frontend assets found — SPA fallback \
             disabled. Local UI dev: run `NO_COLOR=true trunk serve --address 127.0.0.1 --port \
             8080` from apps/web; production: run `trunk build --release` before building deve_cli \
             or set DEVE_STATIC_DIR.",
            dir,
            err
        );
        return Router::new();
    }

    let index = dir.join("index.html");
    tracing::info!("Serving static files from {:?}", dir);

    let fallback = service_fn(move |req| serve_spa_fallback(req, index.clone()));
    let serve = ServeDir::new(&dir).fallback(fallback);

    Router::new().fallback_service(serve)
}

pub(super) fn is_spa_fallback_path(path: &str) -> bool {
    !is_reserved_runtime_path(path)
}

fn is_reserved_runtime_path(path: &str) -> bool {
    path == "/api" || path.starts_with("/api/") || path == "/ws" || path.starts_with("/ws/")
}

async fn serve_spa_fallback<B>(
    req: Request<B>,
    index: PathBuf,
) -> Result<Response<Body>, Infallible> {
    if !is_spa_fallback_path(req.uri().path()) {
        return Ok(not_found_response());
    }

    let response = match tokio::fs::read(&index).await {
        Ok(bytes) => {
            let mut response = Response::new(Body::from(bytes));
            *response.status_mut() = StatusCode::OK;
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            );
            response
        }
        Err(err) => {
            tracing::error!(error = %err, path = ?index, "Failed to read SPA index fallback");
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            response
        }
    };
    Ok(response)
}

fn not_found_response() -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_FOUND;
    response
}

#[cfg(test)]
mod tests;
