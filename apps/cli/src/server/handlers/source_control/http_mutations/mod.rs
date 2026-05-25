// apps/cli/src/server/handlers/source_control/http_mutations/mod.rs
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! # Source Control HTTP Mutation API

mod commit;

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Deserialize;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::handlers::source_control::service;
use crate::server::plugin_host::PluginHostState;
use deve_core::ledger::traits::RepoSelector;
use deve_core::plugin::runtime::host;
use deve_core::protocol::{ScPathTarget, ServerError};

pub use commit::{commit, commit_plugin_host};

#[derive(Deserialize)]
pub struct PathPayload {
    #[serde(default)]
    pub scope_nonce: Option<u64>,
    pub path: String,
    #[serde(default)]
    pub doc_id: Option<deve_core::models::DocId>,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

impl PathPayload {
    fn target(&self) -> ScPathTarget {
        ScPathTarget {
            path: self.path.clone(),
            doc_id: self.doc_id,
        }
    }
}

#[derive(Deserialize)]
pub struct CommitPayload {
    #[serde(default)]
    pub scope_nonce: Option<u64>,
    pub message: String,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

pub async fn stage(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(payload.scope_nonce) {
        return super::errors::http(error);
    }
    let target = payload.target();
    if let Err(error) = ensure_http_selector_writable(&state, &payload.repo) {
        return super::errors::http(error);
    }
    match service::stage_pending(state.repo.as_ref(), &payload.repo, &target) {
        Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn stage_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(payload.scope_nonce) {
        return super::errors::http(error);
    }
    let target = payload.target();
    if let Err(error) = host::ensure_source_control_write_allowed(&payload.repo) {
        return super::errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match service::stage_pending(repo.as_ref(), &payload.repo, &target) {
            Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
            Err(e) => super::errors::http(e),
        },
        Err(e) => super::errors::http(super::errors::unsupported(e.to_string())),
    }
}

pub async fn discard_pending(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(payload.scope_nonce) {
        return super::errors::http(error);
    }
    let target = payload.target();
    if let Err(error) = ensure_http_selector_writable(&state, &payload.repo) {
        return super::errors::http(error);
    }
    match super::local_discard::discard_via_sync_manager(&state, &payload.repo, &target) {
        Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn discard_pending_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(payload.scope_nonce) {
        return super::errors::http(error);
    }
    let target = payload.target();
    if let Err(error) = host::ensure_source_control_write_allowed(&payload.repo) {
        return super::errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match service::discard_pending(repo.as_ref(), &payload.repo, &target) {
            Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
            Err(e) => super::errors::http(e),
        },
        Err(e) => super::errors::http(super::errors::unsupported(e.to_string())),
    }
}

pub async fn unstage(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(payload.scope_nonce) {
        return super::errors::http(error);
    }
    let target = payload.target();
    if let Err(error) = ensure_http_selector_writable(&state, &payload.repo) {
        return super::errors::http(error);
    }
    match service::unstage_file(state.repo.as_ref(), &payload.repo, &target) {
        Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn unstage_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    if let Err(error) = super::http_scope::require(payload.scope_nonce) {
        return super::errors::http(error);
    }
    let target = payload.target();
    if let Err(error) = host::ensure_source_control_write_allowed(&payload.repo) {
        return super::errors::http(error);
    }
    match host::source_control_api() {
        Ok(repo) => match service::unstage_file(repo.as_ref(), &payload.repo, &target) {
            Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
            Err(e) => super::errors::http(e),
        },
        Err(e) => super::errors::http(super::errors::unsupported(e.to_string())),
    }
}

pub(super) fn ensure_http_selector_writable(
    state: &Arc<AppState>,
    selector: &RepoSelector,
) -> Result<(), ServerError> {
    let repo_name = state
        .repo
        .resolve_local_repo_name_for_execution(selector.repo_id, selector.repo_name.as_deref())
        .map_err(super::errors::map_repo_scope_error)?;
    crate::server::repo_scope::ensure_local_repo_projection_writable(state, &repo_name)
}
