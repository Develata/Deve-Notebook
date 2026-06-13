//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 19_plugins#plugin-runtime-boundary
//!
//! Plugin-host source-control mutation compatibility endpoints.

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use std::sync::Arc;

use crate::server::handlers::source_control::{errors, http_scope, service};
use crate::server::plugin_host::PluginHostState;
use deve_core::plugin::runtime::host;

use super::PathPayload;

pub async fn stage_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    if let Err(error) = http_scope::require(payload.scope_nonce) {
        return errors::http(error);
    }
    let target = payload.target();
    if let Err(error) = host::ensure_source_control_write_allowed(&payload.repo) {
        return errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match service::stage_pending(repo.as_ref(), &payload.repo, &target) {
            Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
            Err(e) => errors::http(e),
        },
        Err(e) => errors::http(errors::unsupported(e.to_string())),
    }
}

pub async fn discard_pending_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    if let Err(error) = http_scope::require(payload.scope_nonce) {
        return errors::http(error);
    }
    let target = payload.target();
    if let Err(error) = host::ensure_source_control_write_allowed(&payload.repo) {
        return errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match service::discard_pending(repo.as_ref(), &payload.repo, &target) {
            Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
            Err(e) => errors::http(e),
        },
        Err(e) => errors::http(errors::unsupported(e.to_string())),
    }
}

pub async fn unstage_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    if let Err(error) = http_scope::require(payload.scope_nonce) {
        return errors::http(error);
    }
    let target = payload.target();
    if let Err(error) = host::ensure_source_control_write_allowed(&payload.repo) {
        return errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match service::unstage_file(repo.as_ref(), &payload.repo, &target) {
            Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
            Err(e) => errors::http(e),
        },
        Err(e) => errors::http(errors::unsupported(e.to_string())),
    }
}
