//! plan_ref:
//!   - 03_storage/index#git-ecosystem-coexistence
//!   - 03_storage/repair#backup-export
//!   - 03_storage/projection#projection-contract
//!   - 04_repository#tree-projection-contract
//!   - 05_diff_logic#source-control-runtime

use crate::admin_api::{
    GitStatusResponse, NodeCheckResponse, ProjectionCheckResponse, ScStatusResponse,
};
use crate::export_entries;
use crate::server::AppState;
use crate::server::error_classify::{
    is_db_locked, is_repo_context_invalid, is_repo_not_selected, is_storage_corruption,
    is_storage_not_found,
};
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::node_check::{check_node_consistency, repair_missing_nodes};
use deve_core::ledger::traits::RepoSelector;
use serde::Deserialize;
use std::sync::Arc;

mod dump;

pub use dump::dump;

#[derive(Deserialize)]
pub struct DumpQuery {
    pub path: String,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

#[derive(Deserialize)]
pub struct NodeCheckQuery {
    #[serde(default)]
    pub repair: bool,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

pub async fn export(
    State(state): State<Arc<AppState>>,
    Query(repo): Query<RepoSelector>,
) -> impl IntoResponse {
    let repo_name = match state
        .repo
        .resolve_local_repo_name_for_execution(repo.repo_id, repo.repo_name.as_deref())
    {
        Ok(name) => name,
        Err(err) => return admin_error_response(err, StatusCode::BAD_REQUEST),
    };
    match export_entries::build(&state.repo, &repo_name) {
        Ok(entries) => Json(entries).into_response(),
        Err(err) => admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn node_check(
    State(state): State<Arc<AppState>>,
    Query(query): Query<NodeCheckQuery>,
) -> impl IntoResponse {
    let repo_names = match resolve_target_repos(state.as_ref(), &query.repo, query.repair) {
        Ok(names) => names,
        Err(err) => return admin_error_response(err, StatusCode::BAD_REQUEST),
    };
    let mut reports = Vec::with_capacity(repo_names.len());
    for repo_name in repo_names {
        let report = match state.repo.run_on_local_repo(&repo_name, |db| {
            if query.repair {
                repair_missing_nodes(db)
            } else {
                check_node_consistency(db)
            }
        }) {
            Ok(report) => report,
            Err(err) => {
                return admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR);
            }
        };
        reports.push(NodeCheckResponse {
            repo_name,
            missing_nodes: report.missing_nodes,
            orphan_nodes: report.orphan_nodes,
        });
    }
    Json(reports).into_response()
}

pub async fn projection_check(
    State(state): State<Arc<AppState>>,
    Query(repo): Query<RepoSelector>,
) -> impl IntoResponse {
    let repo_names = match resolve_target_repos(state.as_ref(), &repo, false) {
        Ok(names) => names,
        Err(err) => return admin_error_response(err, StatusCode::BAD_REQUEST),
    };
    let mut reports = Vec::with_capacity(repo_names.len());
    for repo_name in repo_names {
        let diagnostic = match state
            .sync_manager
            .diagnose_projection_local_repo(&repo_name)
        {
            Ok(diagnostic) => diagnostic,
            Err(err) => return admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR),
        };
        reports.push(ProjectionCheckResponse::from_diagnostic(diagnostic));
    }
    Json(reports).into_response()
}

pub async fn sc_status(
    State(state): State<Arc<AppState>>,
    Query(repo): Query<RepoSelector>,
) -> impl IntoResponse {
    let repo_names = match resolve_target_repos(state.as_ref(), &repo, false) {
        Ok(names) => names,
        Err(err) => return admin_error_response(err, StatusCode::BAD_REQUEST),
    };
    let mut reports = Vec::with_capacity(repo_names.len());
    for repo_name in repo_names {
        let staged = match state.repo.list_staged_in_local_repo(&repo_name) {
            Ok(staged) => staged,
            Err(err) => return admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR),
        };
        let unstaged = match state.repo.list_pending_fs_in_local_repo(&repo_name) {
            Ok(unstaged) => unstaged,
            Err(err) => return admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR),
        };
        let confirmed = match state
            .repo
            .list_confirmed_ledger_changes_in_local_repo(&repo_name)
        {
            Ok(confirmed) => confirmed,
            Err(err) => return admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR),
        };
        reports.push(ScStatusResponse {
            repo_name,
            staged,
            unstaged,
            confirmed,
        });
    }
    Json(reports).into_response()
}

pub async fn git_status(
    State(state): State<Arc<AppState>>,
    Query(repo): Query<RepoSelector>,
) -> impl IntoResponse {
    let repo_names = match resolve_target_repos(state.as_ref(), &repo, false) {
        Ok(names) => names,
        Err(err) => return admin_error_response(err, StatusCode::BAD_REQUEST),
    };
    let mut reports = Vec::with_capacity(repo_names.len());
    for repo_name in repo_names {
        let repo_root = match state.repo.local_repo_workspace_root(&repo_name) {
            Ok(root) => root,
            Err(err) => return admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR),
        };
        let status = match deve_core::git_bridge::inspect_repo_root(&repo_root) {
            Ok(status) => status,
            Err(err) => return admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR),
        };
        let summary = match state.repo.run_on_local_repo(&repo_name, |db| {
            Ok(deve_core::git_bridge::summarize_records(db)?)
        }) {
            Ok(summary) => summary,
            Err(err) => return admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR),
        };
        let records = match state.repo.run_on_local_repo(&repo_name, |db| {
            Ok(deve_core::git_bridge::list_records(db)?)
        }) {
            Ok(records) => records,
            Err(err) => return admin_error_response(err, StatusCode::INTERNAL_SERVER_ERROR),
        };
        reports.push(GitStatusResponse {
            repo_name,
            status,
            summary,
            records,
        });
    }
    Json(reports).into_response()
}

fn resolve_target_repos(
    state: &AppState,
    repo: &RepoSelector,
    repair: bool,
) -> anyhow::Result<Vec<String>> {
    if repo.repo_id.is_some() || repo.repo_name.is_some() {
        return Ok(vec![state.repo.resolve_local_repo_name_for_execution(
            repo.repo_id,
            repo.repo_name.as_deref(),
        )?]);
    }
    if repair {
        anyhow::bail!("Repository selector required for repair node-check");
    }
    state.repo.list_repos(None)
}

pub(super) fn admin_error_response(
    error: impl ToString,
    fallback: StatusCode,
) -> axum::response::Response {
    let detail = error.to_string();
    (classify_admin_error(&detail, fallback), detail).into_response()
}

pub(super) fn classify_admin_error(detail: &str, fallback: StatusCode) -> StatusCode {
    let lower = detail.to_ascii_lowercase();
    if is_storage_not_found(&lower) {
        return StatusCode::NOT_FOUND;
    }
    if is_db_locked(&lower) {
        return StatusCode::SERVICE_UNAVAILABLE;
    }
    if is_storage_corruption(&lower) {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if is_repo_not_selected(&lower) || is_repo_context_invalid(&lower) {
        return StatusCode::CONFLICT;
    }
    fallback
}

#[cfg(test)]
mod tests;
