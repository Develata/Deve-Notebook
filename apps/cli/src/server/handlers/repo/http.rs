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
        Err(e) => http_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            ServerErrorCode::RequestFailed,
            e.to_string(),
        ),
    }
}

pub async fn list_docs_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(repo): Query<RepoSelector>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repository) => match repository.list_docs_in_repo(&repo) {
            Ok(list) => Json(list).into_response(),
            Err(e) => http_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ServerErrorCode::RequestFailed,
                e.to_string(),
            ),
        },
        Err(e) => http_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            ServerErrorCode::RequestFailed,
            e.to_string(),
        ),
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
        Err(e) => http_error(
            StatusCode::NOT_FOUND,
            ServerErrorCode::StorageNotFound,
            e.to_string(),
        ),
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
            Err(e) => http_error(
                StatusCode::NOT_FOUND,
                ServerErrorCode::StorageNotFound,
                e.to_string(),
            ),
        },
        Err(e) => http_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            ServerErrorCode::RequestFailed,
            e.to_string(),
        ),
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

#[cfg(test)]
mod tests {
    use super::parse_doc_id;

    #[test]
    fn invalid_doc_id_returns_small_error_detail() {
        assert_eq!(
            parse_doc_id("not-a-uuid").expect_err("must reject"),
            "invalid doc_id"
        );
    }
}
