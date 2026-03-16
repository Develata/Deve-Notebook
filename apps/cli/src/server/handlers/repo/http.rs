// apps/cli/src/server/handlers/repo/http.rs
//! # Repo HTTP API

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use deve_core::protocol::{ServerError, ServerErrorCode};
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
        Err(e) => repo_error_response(e),
    }
}

pub async fn list_docs_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(repo): Query<RepoSelector>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repository) => match repository.list_docs_in_repo(&repo) {
            Ok(list) => Json(list).into_response(),
            Err(e) => repo_error_response(e),
        },
        Err(e) => repo_error_response(e),
    }
}

pub async fn doc_content(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DocQuery>,
) -> impl IntoResponse {
    let doc_id = match parse_doc_id(&q.doc_id) {
        Ok(doc_id) => doc_id,
        Err(detail) => {
            return http_error(
                StatusCode::BAD_REQUEST,
                ServerErrorCode::RequestFailed,
                detail,
            );
        }
    };
    match Repository::get_doc_content_in_repo(state.repo.as_ref(), &q.repo, doc_id) {
        Ok(content) => content.into_response(),
        Err(e) => repo_error_response(e),
    }
}

pub async fn doc_content_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<DocQuery>,
) -> impl IntoResponse {
    let doc_id = match parse_doc_id(&q.doc_id) {
        Ok(doc_id) => doc_id,
        Err(detail) => {
            return http_error(
                StatusCode::BAD_REQUEST,
                ServerErrorCode::RequestFailed,
                detail,
            );
        }
    };
    match host::repository() {
        Ok(repo) => match repo.get_doc_content_in_repo(&q.repo, doc_id) {
            Ok(content) => content.into_response(),
            Err(e) => repo_error_response(e),
        },
        Err(e) => repo_error_response(e),
    }
}

fn parse_doc_id(raw: &str) -> Result<DocId, &'static str> {
    uuid::Uuid::parse_str(raw)
        .map(DocId)
        .map_err(|_| "invalid doc_id")
}

fn http_error(
    status: StatusCode,
    code: ServerErrorCode,
    detail: impl Into<String>,
) -> axum::response::Response {
    (status, Json(ServerError::with_detail(code, detail))).into_response()
}

fn repo_error_response(error: impl ToString) -> axum::response::Response {
    let detail = error.to_string();
    let (status, code) = classify_repo_error(&detail);
    http_error(status, code, detail)
}

fn classify_repo_error(detail: &str) -> (StatusCode, ServerErrorCode) {
    let lower = detail.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "repository not found:",
            "document not found",
            "doc not found",
            "local repo not found for name",
        ],
    ) {
        return (StatusCode::NOT_FOUND, ServerErrorCode::StorageNotFound);
    }
    if contains_any(
        &lower,
        &[
            "database already open",
            "cannot acquire lock",
            "db locked",
            "database is locked",
        ],
    ) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            ServerErrorCode::StorageDbLocked,
        );
    }
    if lower.contains("tracked document projection missing") {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            ServerErrorCode::StoragePersistFailed,
        );
    }
    if contains_any(
        &lower,
        &[
            "active repository not selected",
            "multiple local repos exist",
            "no local repositories available",
        ],
    ) {
        return (StatusCode::CONFLICT, ServerErrorCode::SyncRepoUnbound);
    }
    if contains_any(
        &lower,
        &[
            "remote session lost repo name",
            "cannot bootstrap local repo while on remote branch",
            "repository uuid not resolved",
            "remote repository selector not resolved",
            "local repository selector not resolved",
            "local repository uuid not resolved",
            "session repo mismatch",
            "repo selector mismatch",
            "ambiguous local repository selector",
            "ambiguous remote repository selector",
            "local repo not found for uuid",
            "local repo operation requested on remote branch",
            "local workspace path requested on remote branch",
            "local workspace root requested on remote branch",
            "scope mismatch",
            "stale scope nonce",
        ],
    ) {
        return (StatusCode::CONFLICT, ServerErrorCode::ScRepoContextInvalid);
    }
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        ServerErrorCode::RequestFailed,
    )
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}

#[cfg(test)]
#[path = "http_test.rs"]
mod tests;
