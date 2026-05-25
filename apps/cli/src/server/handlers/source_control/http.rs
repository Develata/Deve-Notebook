// apps/cli/src/server/handlers/source_control/http.rs
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! # Source Control HTTP Query API

use axum::Json;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use serde::Deserialize;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::handlers::source_control::service;
use crate::server::plugin_host::PluginHostState;
use deve_core::git_bridge::GitMirrorRepairReview;
use deve_core::ledger::traits::RepoSelector;
use deve_core::plugin::runtime::host;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeEntry;

#[derive(Deserialize, Default)]
pub struct RepoQuery {
    #[serde(default)]
    pub scope_nonce: Option<u64>,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

#[derive(Deserialize)]
pub struct DiffQuery {
    #[serde(default)]
    pub scope_nonce: Option<u64>,
    pub path: String,
    #[serde(default)]
    pub doc_id: Option<deve_core::models::DocId>,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

impl DiffQuery {
    fn target(&self) -> ScPathTarget {
        ScPathTarget {
            path: self.path.clone(),
            doc_id: self.doc_id,
        }
    }
}

pub async fn pending(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RepoQuery>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(q.scope_nonce) {
        return super::errors::http(error);
    }
    match service::list_pending(state.repo.as_ref(), &q.repo) {
        Ok(changes) => Json::<Vec<ChangeEntry>>(changes).into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn pending_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<RepoQuery>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(q.scope_nonce) {
        return super::errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match service::list_pending(repo.as_ref(), &q.repo) {
            Ok(changes) => Json::<Vec<ChangeEntry>>(changes).into_response(),
            Err(e) => super::errors::http(e),
        },
        Err(e) => super::errors::http(super::errors::unsupported(e.to_string())),
    }
}

pub async fn status(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RepoQuery>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(q.scope_nonce) {
        return super::errors::http(error);
    }
    match service::list_changes(state.repo.as_ref(), &q.repo) {
        Ok(changes) => Json::<Vec<ChangeEntry>>(changes).into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn staged(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RepoQuery>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(q.scope_nonce) {
        return super::errors::http(error);
    }
    match deve_core::source_control::SourceControlApi::list_staged_in_repo(
        state.repo.as_ref(),
        &q.repo,
    ) {
        Ok(changes) => {
            Json::<Vec<ChangeEntry>>(super::present::collapse_rename_candidates(changes))
                .into_response()
        }
        Err(e) => super::errors::http(super::errors::map_repo_error(
            super::errors::ScOp::ListChanges,
            e,
        )),
    }
}

pub async fn git_mirror_repair_review(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RepoQuery>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(q.scope_nonce) {
        return super::errors::http(error);
    }
    match service::git_mirror_repair_review(state.repo.as_ref(), &q.repo) {
        Ok(review) => Json::<GitMirrorRepairReview>(review).into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn status_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<RepoQuery>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(q.scope_nonce) {
        return super::errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match service::list_changes(repo.as_ref(), &q.repo) {
            Ok(changes) => Json::<Vec<ChangeEntry>>(changes).into_response(),
            Err(e) => super::errors::http(e),
        },
        Err(e) => super::errors::http(super::errors::unsupported(e.to_string())),
    }
}

pub async fn staged_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<RepoQuery>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(q.scope_nonce) {
        return super::errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match repo.list_staged_in_repo(&q.repo) {
            Ok(changes) => {
                Json::<Vec<ChangeEntry>>(super::present::collapse_rename_candidates(changes))
                    .into_response()
            }
            Err(e) => super::errors::http(super::errors::map_repo_error(
                super::errors::ScOp::ListChanges,
                e,
            )),
        },
        Err(e) => super::errors::http(super::errors::unsupported(e.to_string())),
    }
}

pub async fn diff(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DiffQuery>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(q.scope_nonce) {
        return super::errors::http(error);
    }
    match service::diff_doc_target(state.repo.as_ref(), &q.repo, &q.target()) {
        Ok(diff) => diff.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn diff_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<DiffQuery>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(q.scope_nonce) {
        return super::errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match service::diff_doc_target(repo.as_ref(), &q.repo, &q.target()) {
            Ok(diff) => diff.into_response(),
            Err(e) => super::errors::http(e),
        },
        Err(e) => super::errors::http(super::errors::unsupported(e.to_string())),
    }
}
