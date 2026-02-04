// apps/cli/src/server/handlers/repo/http.rs
//! # Repo HTTP API

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::handlers;
use crate::server::plugin_host::PluginHostState;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::DocId;
use deve_core::models::RepoType;
use deve_core::plugin::runtime::host;
use deve_core::state::reconstruct_content;

#[derive(Deserialize)]
pub struct DocQuery {
    pub doc_id: String,
}

pub async fn list_docs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let repo_id = handlers::get_repo_id(&state);
    match state.repo.list_docs(&RepoType::Local(repo_id)) {
        Ok(list) => Json(list).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn list_docs_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match repo.list_docs() {
            Ok(list) => Json(list).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn doc_content(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DocQuery>,
) -> impl IntoResponse {
    let uuid = match uuid::Uuid::parse_str(&q.doc_id) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid doc_id").into_response(),
    };
    let doc_id = DocId(uuid);
    match state.repo.get_local_ops(doc_id) {
        Ok(ops) => {
            let entries: Vec<_> = ops.into_iter().map(|(_, e)| e).collect();
            let content = reconstruct_content(&entries);
            content.into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "doc not found").into_response(),
    }
}

pub async fn doc_content_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<DocQuery>,
) -> impl IntoResponse {
    let uuid = match uuid::Uuid::parse_str(&q.doc_id) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid doc_id").into_response(),
    };
    let doc_id = DocId(uuid);
    match host::repository() {
        Ok(repo) => match repo.get_doc_content(doc_id) {
            Ok(content) => content.into_response(),
            Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
