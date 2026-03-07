// apps/cli/src/server/handlers/repo/http.rs
//! # Repo HTTP API

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::plugin_host::PluginHostState;
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::models::DocId;
use deve_core::plugin::runtime::host;

#[derive(Deserialize)]
pub struct DocQuery {
    pub doc_id: String,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

pub async fn list_docs(
    State(state): State<Arc<AppState>>,
    Query(repo): Query<RepoSelector>,
) -> impl IntoResponse {
    match Repository::list_docs_in_repo(state.repo.as_ref(), &repo) {
        Ok(list) => Json(list).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn list_docs_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(repo): Query<RepoSelector>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repository) => match repository.list_docs_in_repo(&repo) {
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
    match Repository::get_doc_content_in_repo(state.repo.as_ref(), &q.repo, doc_id) {
        Ok(content) => content.into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
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
        Ok(repo) => match repo.get_doc_content_in_repo(&q.repo, doc_id) {
            Ok(content) => content.into_response(),
            Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
