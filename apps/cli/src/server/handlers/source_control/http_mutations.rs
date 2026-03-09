// apps/cli/src/server/handlers/source_control/http_mutations.rs
//! # Source Control HTTP Mutation API

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::handlers::source_control::service;
use crate::server::plugin_host::PluginHostState;
use deve_core::ledger::traits::RepoSelector;
use deve_core::plugin::runtime::host;
use deve_core::source_control::CommitInfo;

#[derive(Deserialize)]
pub struct PathPayload {
    pub path: String,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

#[derive(Deserialize)]
pub struct CommitPayload {
    pub message: String,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

pub async fn stage(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    match service::stage_pending(state.repo.as_ref(), &payload.repo, &payload.path) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn stage_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match service::stage_pending(repo.as_ref(), &payload.repo, &payload.path) {
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn discard_pending(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    match service::discard_pending(state.repo.as_ref(), &payload.repo, &payload.path) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn discard_pending_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match service::discard_pending(repo.as_ref(), &payload.repo, &payload.path) {
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn unstage(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    match service::unstage_file(state.repo.as_ref(), &payload.repo, &payload.path) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn unstage_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match service::unstage_file(repo.as_ref(), &payload.repo, &payload.path) {
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn commit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CommitPayload>,
) -> impl IntoResponse {
    match service::commit_staged(state.repo.as_ref(), &payload.repo, &payload.message) {
        Ok(info) => Json::<CommitInfo>(info).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn commit_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<CommitPayload>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match service::commit_staged(repo.as_ref(), &payload.repo, &payload.message) {
            Ok(info) => Json::<CommitInfo>(info).into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
