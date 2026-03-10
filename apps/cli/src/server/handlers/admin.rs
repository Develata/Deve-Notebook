use crate::admin_api::NodeCheckResponse;
use crate::export_entries;
use crate::server::AppState;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::node_check::{check_node_consistency, repair_missing_nodes};
use deve_core::ledger::traits::RepoSelector;
use serde::Deserialize;
use std::sync::Arc;

#[path = "admin_dump.rs"]
mod admin_dump;

pub use admin_dump::dump;

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
        .resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())
    {
        Ok(name) => name,
        Err(err) => return (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    };
    match export_entries::build(&state.repo, &repo_name) {
        Ok(entries) => Json(entries).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to export ledger facts".to_string(),
        )
            .into_response(),
    }
}

pub async fn node_check(
    State(state): State<Arc<AppState>>,
    Query(query): Query<NodeCheckQuery>,
) -> impl IntoResponse {
    let repo_names = match resolve_target_repos(state.as_ref(), &query.repo) {
        Ok(names) => names,
        Err(err) => return (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    };
    let mut reports = Vec::with_capacity(repo_names.len());
    for repo_name in repo_names {
        let result = state.repo.run_on_local_repo(&repo_name, |db| {
            if query.repair {
                repair_missing_nodes(db)
            } else {
                check_node_consistency(db)
            }
        });
        let Ok(report) = result else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to inspect repo consistency".to_string(),
            )
                .into_response();
        };
        reports.push(NodeCheckResponse {
            repo_name,
            missing_nodes: report.missing_nodes,
            orphan_nodes: report.orphan_nodes,
        });
    }
    Json(reports).into_response()
}

fn resolve_target_repos(state: &AppState, repo: &RepoSelector) -> anyhow::Result<Vec<String>> {
    if repo.repo_id.is_some() || repo.repo_name.is_some() {
        return Ok(vec![state.repo.resolve_local_repo_name(
            repo.repo_id,
            repo.repo_name.as_deref(),
        )?]);
    }
    state.repo.list_repos(None)
}
