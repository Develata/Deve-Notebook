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
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::source_control_grants::AuthSessionId;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeDomain;

use authority::{AuthorizedRepoBinding, SourceControlWriteAuthority, authorize_http_write};
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
    let binding = match authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::BrowserSessionGrant(&auth_session_id),
    ) {
        Ok(binding) => binding,
        Err(error) => return super::errors::http(error),
    };
    let pinned = binding.pinned_selector();
    let repo_name = match state
        .repo
        .resolve_local_repo_name_for_execution(pinned.repo_id, pinned.repo_name.as_deref())
    {
        Ok(repo_name) => repo_name,
        Err(error) => {
            return super::errors::http(super::errors::map_repo_error(
                super::errors::ScOp::ApplyExternalChanges,
                error,
            ));
        }
    };
    let prepared = match state
        .repo
        .prepare_external_changes_in_local_repo(&repo_name)
    {
        Ok(prepared) => prepared,
        Err(error) => {
            return super::errors::http(super::errors::map_repo_error(
                super::errors::ScOp::ApplyExternalChanges,
                error,
            ));
        }
    };
    match state
        .repo_mutation_gate()
        .execute_repo(binding.repo_id, &state.tx, || {
            let repo_name = match crate::server::repo_mutation::revalidate_writable_local_repo(
                &state,
                binding.repo_id,
                &repo_name,
            ) {
                Ok(repo_name) => repo_name,
                Err(error) => {
                    return MutationExecution::not_committed(super::errors::map_repo_error(
                        super::errors::ScOp::ApplyExternalChanges,
                        error,
                    ));
                }
            };
            match state
                .repo
                .commit_prepared_external_changes_in_local_repo(&repo_name, prepared)
                .map_err(|error| {
                    super::errors::map_repo_error(super::errors::ScOp::ApplyExternalChanges, error)
                }) {
                Ok(outcome) => {
                    let receipt = outcome.receipt;
                    MutationExecution::committed(
                        receipt.clone(),
                        MutationPublication::external_apply_recovery(
                            receipt.repo_id,
                            receipt.affected_docs.clone(),
                        ),
                    )
                }
                Err(error) => MutationExecution::not_committed(error),
            }
        })
        .await
    {
        Ok(MutationExecution::Committed { value: receipt, .. }) => Json(receipt).into_response(),
        Ok(MutationExecution::NotCommitted(error)) => super::errors::http(error),
        Ok(MutationExecution::ProjectionDegraded { value: receipt, .. }) => {
            Json(receipt).into_response()
        }
        Ok(MutationExecution::CommittedPartial { error, .. }) => super::errors::http(error),
        Err(error) => super::errors::http(deve_core::protocol::ServerError::with_detail(
            deve_core::protocol::ServerErrorCode::StoragePersistFailed,
            error.to_string(),
        )),
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
    let binding = match authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::DelegatedRemoteProxy,
    ) {
        Ok(binding) => binding,
        Err(error) => return super::errors::http(error),
    };
    let pinned = binding.pinned_selector();
    let repo_name = match state
        .repo
        .resolve_local_repo_name_for_execution(pinned.repo_id, pinned.repo_name.as_deref())
    {
        Ok(repo_name) => repo_name,
        Err(error) => {
            return super::errors::http(super::errors::map_repo_error(
                super::errors::ScOp::ApplyExternalChanges,
                error,
            ));
        }
    };
    let prepared = match state
        .repo
        .prepare_external_changes_in_local_repo(&repo_name)
    {
        Ok(prepared) => prepared,
        Err(error) => {
            return super::errors::http(super::errors::map_repo_error(
                super::errors::ScOp::ApplyExternalChanges,
                error,
            ));
        }
    };
    match state
        .repo_mutation_gate()
        .execute_repo(binding.repo_id, &state.tx, || {
            let repo_name = match crate::server::repo_mutation::revalidate_writable_local_repo(
                &state,
                binding.repo_id,
                &repo_name,
            ) {
                Ok(repo_name) => repo_name,
                Err(error) => {
                    return MutationExecution::not_committed(super::errors::map_repo_error(
                        super::errors::ScOp::ApplyExternalChanges,
                        error,
                    ));
                }
            };
            match state
                .repo
                .commit_prepared_external_changes_in_local_repo(&repo_name, prepared)
                .map_err(|error| {
                    super::errors::map_repo_error(super::errors::ScOp::ApplyExternalChanges, error)
                }) {
                Ok(outcome) => {
                    let receipt = outcome.receipt;
                    MutationExecution::committed(
                        receipt.clone(),
                        MutationPublication::external_apply_recovery(
                            receipt.repo_id,
                            receipt.affected_docs.clone(),
                        ),
                    )
                }
                Err(error) => MutationExecution::not_committed(error),
            }
        })
        .await
    {
        Ok(MutationExecution::Committed { value: receipt, .. }) => Json(receipt).into_response(),
        Ok(MutationExecution::NotCommitted(error)) => super::errors::http(error),
        Ok(MutationExecution::ProjectionDegraded { value: receipt, .. }) => {
            Json(receipt).into_response()
        }
        Ok(MutationExecution::CommittedPartial { error, .. }) => super::errors::http(error),
        Err(error) => super::errors::http(deve_core::protocol::ServerError::with_detail(
            deve_core::protocol::ServerErrorCode::StoragePersistFailed,
            error.to_string(),
        )),
    }
}
