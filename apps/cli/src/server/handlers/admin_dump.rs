//! plan_ref:
//!   - 04_storage#backup-export
//!   - 04_storage#facts-partition

use crate::dump_support;
use crate::server::AppState;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;

use super::DumpQuery;

pub async fn dump(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DumpQuery>,
) -> impl IntoResponse {
    let repo_name = match state
        .repo
        .resolve_local_repo_name_for_execution(query.repo.repo_id, query.repo.repo_name.as_deref())
    {
        Ok(name) => name,
        Err(err) => return super::admin_error_response(err, StatusCode::BAD_REQUEST),
    };
    match dump_support::build_dump(&state.repo, &repo_name, &query.path) {
        Ok(Some(dump)) => Json(dump).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Path not found in Ledger.").into_response(),
        Err(err) => super::admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR),
    }
}
