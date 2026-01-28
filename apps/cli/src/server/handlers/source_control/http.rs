// apps/cli/src/server/handlers/source_control/http.rs
//! # Source Control HTTP API

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::plugin_host::PluginHostState;
use deve_core::plugin::runtime::host;
use deve_core::source_control::{ChangeEntry, CommitInfo};

#[derive(Deserialize)]
pub struct DiffQuery {
    pub path: String,
}

#[derive(Deserialize)]
pub struct PathPayload {
    pub path: String,
}

#[derive(Deserialize)]
pub struct CommitPayload {
    pub message: String,
}

pub async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.repo.list_changes() {
        Ok(changes) => Json::<Vec<ChangeEntry>>(changes).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn status_plugin_host(State(_state): State<Arc<PluginHostState>>) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match repo.list_changes() {
            Ok(changes) => Json::<Vec<ChangeEntry>>(changes).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn diff(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DiffQuery>,
) -> impl IntoResponse {
    match state.repo.diff_doc_path(&q.path) {
        Ok(diff) => diff.into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

pub async fn diff_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<DiffQuery>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match repo.diff_doc_path(&q.path) {
            Ok(diff) => diff.into_response(),
            Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn stage(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    match state.repo.stage_file(&payload.path) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn stage_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match repo.stage_file(&payload.path) {
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
    match state.repo.commit_staged(&payload.message) {
        Ok(info) => Json::<CommitInfo>(info).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn commit_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<CommitPayload>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match repo.commit_staged(&payload.message) {
            Ok(info) => Json::<CommitInfo>(info).into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
