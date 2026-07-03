// apps/cli/src/server/handlers/source_control/http_mutations/mod.rs
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! # Source Control HTTP Mutation API

mod authority;
mod commit;
mod plugin_host;

use axum::Extension;
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Deserialize;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::handlers::source_control::service;
use crate::server::source_control_grants::AuthSessionId;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeDomain;

use authority::{SourceControlWriteAuthority, authorize_http_write};
pub(crate) use commit::commit;
pub use commit::{commit_delegated, commit_plugin_host};
pub use plugin_host::{discard_pending_plugin_host, stage_plugin_host, unstage_plugin_host};

#[derive(Deserialize)]
pub struct PathPayload {
    #[serde(default)]
    pub scope_nonce: Option<u64>,
    pub path: String,
    #[serde(default)]
    pub doc_id: Option<deve_core::models::DocId>,
    #[serde(default)]
    pub domain: Option<ChangeDomain>,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

impl PathPayload {
    fn target(&self) -> ScPathTarget {
        ScPathTarget {
            path: self.path.clone(),
            doc_id: self.doc_id,
            domain: self.domain,
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

#[derive(Deserialize)]
pub struct ApplyExternalPayload {
    #[serde(default)]
    pub scope_nonce: Option<u64>,
    #[serde(flatten)]
    pub repo: RepoSelector,
}

pub(crate) async fn stage(
    State(state): State<Arc<AppState>>,
    Extension(auth_session_id): Extension<AuthSessionId>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    let scope_nonce = match super::http_scope::require(payload.scope_nonce) {
        Ok(scope_nonce) => scope_nonce,
        Err(error) => return super::errors::http(error),
    };
    if let Err(error) = authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::BrowserSessionGrant(&auth_session_id),
    ) {
        return super::errors::http(error);
    }
    let target = payload.target();
    match service::stage_pending(state.repo.as_ref(), &payload.repo, &target) {
        Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn stage_delegated(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    let scope_nonce = match super::http_scope::require(payload.scope_nonce) {
        Ok(scope_nonce) => scope_nonce,
        Err(error) => return super::errors::http(error),
    };
    if let Err(error) = authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::DelegatedRemoteProxy,
    ) {
        return super::errors::http(error);
    }
    let target = payload.target();
    match service::stage_pending(state.repo.as_ref(), &payload.repo, &target) {
        Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub(crate) async fn discard_pending(
    State(state): State<Arc<AppState>>,
    Extension(auth_session_id): Extension<AuthSessionId>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    let scope_nonce = match super::http_scope::require(payload.scope_nonce) {
        Ok(scope_nonce) => scope_nonce,
        Err(error) => return super::errors::http(error),
    };
    if let Err(error) = authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::BrowserSessionGrant(&auth_session_id),
    ) {
        return super::errors::http(error);
    }
    let target = payload.target();
    match super::local_discard::discard_via_sync_manager(&state, &payload.repo, &target) {
        Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn discard_pending_delegated(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    let scope_nonce = match super::http_scope::require(payload.scope_nonce) {
        Ok(scope_nonce) => scope_nonce,
        Err(error) => return super::errors::http(error),
    };
    if let Err(error) = authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::DelegatedRemoteProxy,
    ) {
        return super::errors::http(error);
    }
    let target = payload.target();
    match super::local_discard::discard_via_sync_manager(&state, &payload.repo, &target) {
        Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub(crate) async fn unstage(
    State(state): State<Arc<AppState>>,
    Extension(auth_session_id): Extension<AuthSessionId>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    let scope_nonce = match super::http_scope::require(payload.scope_nonce) {
        Ok(scope_nonce) => scope_nonce,
        Err(error) => return super::errors::http(error),
    };
    if let Err(error) = authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::BrowserSessionGrant(&auth_session_id),
    ) {
        return super::errors::http(error);
    }
    let target = payload.target();
    match service::unstage_file(state.repo.as_ref(), &payload.repo, &target) {
        Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn unstage_delegated(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PathPayload>,
) -> impl IntoResponse {
    let scope_nonce = match super::http_scope::require(payload.scope_nonce) {
        Ok(scope_nonce) => scope_nonce,
        Err(error) => return super::errors::http(error),
    };
    if let Err(error) = authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::DelegatedRemoteProxy,
    ) {
        return super::errors::http(error);
    }
    let target = payload.target();
    match service::unstage_file(state.repo.as_ref(), &payload.repo, &target) {
        Ok(_) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub(crate) async fn apply_external_changes(
    State(state): State<Arc<AppState>>,
    Extension(auth_session_id): Extension<AuthSessionId>,
    Json(payload): Json<ApplyExternalPayload>,
) -> impl IntoResponse {
    let scope_nonce = match super::http_scope::require(payload.scope_nonce) {
        Ok(scope_nonce) => scope_nonce,
        Err(error) => return super::errors::http(error),
    };
    if let Err(error) = authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::BrowserSessionGrant(&auth_session_id),
    ) {
        return super::errors::http(error);
    }
    match service::apply_external_changes(state.repo.as_ref(), &payload.repo) {
        Ok(changes) => Json(changes).into_response(),
        Err(e) => super::errors::http(e),
    }
}

pub async fn apply_external_changes_delegated(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ApplyExternalPayload>,
) -> impl IntoResponse {
    let scope_nonce = match super::http_scope::require(payload.scope_nonce) {
        Ok(scope_nonce) => scope_nonce,
        Err(error) => return super::errors::http(error),
    };
    if let Err(error) = authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::DelegatedRemoteProxy,
    ) {
        return super::errors::http(error);
    }
    match service::apply_external_changes(state.repo.as_ref(), &payload.repo) {
        Ok(changes) => Json(changes).into_response(),
        Err(e) => super::errors::http(e),
    }
}
