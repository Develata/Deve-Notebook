// apps/cli/src/server/handlers/source_control/http_commits.rs
//! # Source Control HTTP Commit Queries

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::handlers::source_control::service;
use crate::server::plugin_host::PluginHostState;
use deve_core::ledger::traits::RepoSelector;
use deve_core::plugin::runtime::host;
use deve_core::source_control::{CommitFileDiff, CommitInfo};

#[derive(Deserialize)]
pub struct CommitHistoryQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

#[derive(Deserialize)]
pub struct CommitDiffQuery {
    pub commit_b: String,
    #[serde(default)]
    pub commit_a: Option<String>,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

const fn default_limit() -> u32 {
    50
}

pub async fn commit_history(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CommitHistoryQuery>,
) -> impl IntoResponse {
    match service::list_commit_history(state.repo.as_ref(), &q.repo, q.limit) {
        Ok(commits) => Json::<Vec<CommitInfo>>(commits).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn commit_history_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<CommitHistoryQuery>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match service::list_commit_history(repo.as_ref(), &q.repo, q.limit) {
            Ok(commits) => Json::<Vec<CommitInfo>>(commits).into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn commit_diff(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CommitDiffQuery>,
) -> impl IntoResponse {
    match service::diff_commits(
        state.repo.as_ref(),
        &q.repo,
        q.commit_a.as_deref(),
        &q.commit_b,
    ) {
        Ok(diffs) => Json::<Vec<CommitFileDiff>>(diffs).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn commit_diff_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<CommitDiffQuery>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => {
            match service::diff_commits(repo.as_ref(), &q.repo, q.commit_a.as_deref(), &q.commit_b)
            {
                Ok(diffs) => Json::<Vec<CommitFileDiff>>(diffs).into_response(),
                Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
