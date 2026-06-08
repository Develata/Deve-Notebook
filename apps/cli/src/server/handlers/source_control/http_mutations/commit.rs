//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::handlers::source_control::errors;
use crate::server::handlers::source_control::service;
use crate::server::plugin_host::PluginHostState;
use deve_core::plugin::runtime::host;
use deve_core::source_control::CommitInfo;

use super::CommitPayload;

pub async fn commit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CommitPayload>,
) -> impl IntoResponse {
    if let Err(error) = super::super::http_scope::require(payload.scope_nonce) {
        return errors::http(error);
    }
    if let Err(error) = super::ensure_http_selector_writable(&state, &payload.repo) {
        return errors::http(error);
    }
    match service::commit_staged_with_git_bridge(
        state.repo.as_ref(),
        &payload.repo,
        &payload.message,
        state.git_bridge,
    ) {
        Ok(info) => Json::<CommitInfo>(info).into_response(),
        Err(e) => errors::http(e),
    }
}

pub async fn commit_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<CommitPayload>,
) -> impl IntoResponse {
    if let Err(error) = super::super::http_scope::require(payload.scope_nonce) {
        return errors::http(error);
    }
    if let Err(error) = host::ensure_source_control_write_allowed(&payload.repo) {
        return errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match service::commit_staged(repo.as_ref(), &payload.repo, &payload.message) {
            Ok(info) => Json::<CommitInfo>(info).into_response(),
            Err(e) => errors::http(e),
        },
        Err(e) => errors::http(errors::unsupported(e.to_string())),
    }
}
