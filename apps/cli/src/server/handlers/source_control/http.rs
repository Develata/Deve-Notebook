// apps/cli/src/server/handlers/source_control/http.rs
//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-scope-runtime
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
use deve_core::ledger::traits::RepoSelector;
use deve_core::plugin::runtime::host;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeEntry;

#[derive(Deserialize, Default)]
pub struct RepoQuery {
    #[serde(flatten)]
    pub repo: RepoSelector,
}

#[derive(Deserialize)]
pub struct DiffQuery {
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
    match service::list_pending(state.repo.as_ref(), &q.repo) {
        Ok(changes) => Json::<Vec<ChangeEntry>>(changes).into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn pending_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<RepoQuery>,
) -> impl IntoResponse {
    match host::repository() {
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
    match service::list_changes(state.repo.as_ref(), &q.repo) {
        Ok(changes) => Json::<Vec<ChangeEntry>>(changes).into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn status_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<RepoQuery>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match service::list_changes(repo.as_ref(), &q.repo) {
            Ok(changes) => Json::<Vec<ChangeEntry>>(changes).into_response(),
            Err(e) => super::errors::http(e),
        },
        Err(e) => super::errors::http(super::errors::unsupported(e.to_string())),
    }
}

pub async fn diff(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DiffQuery>,
) -> impl IntoResponse {
    match service::diff_doc_target(state.repo.as_ref(), &q.repo, &q.target()) {
        Ok(diff) => diff.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn diff_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Query(q): Query<DiffQuery>,
) -> impl IntoResponse {
    match host::repository() {
        Ok(repo) => match service::diff_doc_target(repo.as_ref(), &q.repo, &q.target()) {
            Ok(diff) => diff.into_response(),
            Err(e) => super::errors::http(e),
        },
        Err(e) => super::errors::http(super::errors::unsupported(e.to_string())),
    }
}
