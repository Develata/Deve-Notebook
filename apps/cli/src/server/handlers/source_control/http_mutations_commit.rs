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
    match service::commit_staged(state.repo.as_ref(), &payload.repo, &payload.message) {
        Ok(info) => Json::<CommitInfo>(info).into_response(),
        Err(e) => errors::http(e),
    }
}

pub async fn commit_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<CommitPayload>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match service::commit_staged(repo.as_ref(), &payload.repo, &payload.message) {
            Ok(info) => Json::<CommitInfo>(info).into_response(),
            Err(e) => errors::http(e),
        },
        Err(e) => errors::http(errors::unsupported(e.to_string())),
    }
}
