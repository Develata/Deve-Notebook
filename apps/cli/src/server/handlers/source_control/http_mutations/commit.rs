//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime

use axum::Extension;
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::handlers::source_control::errors;
use crate::server::plugin_host::PluginHostState;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::source_control_grants::AuthSessionId;
use deve_core::plugin::runtime::host;
use deve_core::source_control::CommitInfo;

use super::{AuthorizedRepoBinding, CommitPayload, SourceControlWriteAuthority};

pub(crate) async fn commit(
    State(state): State<Arc<AppState>>,
    Extension(auth_session_id): Extension<AuthSessionId>,
    Json(payload): Json<CommitPayload>,
) -> impl IntoResponse {
    let scope_nonce = match super::super::http_scope::require(payload.scope_nonce) {
        Ok(scope_nonce) => scope_nonce,
        Err(error) => return errors::http(error),
    };
    let binding = match super::authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::BrowserSessionGrant(&auth_session_id),
    ) {
        Ok(binding) => binding,
        Err(error) => return errors::http(error),
    };
    commit_through_gate(&state, binding, payload).await
}

async fn commit_through_gate(
    state: &Arc<AppState>,
    binding: AuthorizedRepoBinding,
    payload: CommitPayload,
) -> axum::response::Response {
    let repo_id = binding.repo_id;
    let pinned = binding.pinned_selector();
    let expected_name = pinned.repo_name.as_deref().unwrap_or_default();
    let repo_name = match crate::server::repo_mutation::prepare_writable_local_repo(
        state,
        repo_id,
        expected_name,
    ) {
        Ok(name) => name,
        Err(error) => {
            return errors::http(errors::map_repo_error(errors::ScOp::Commit, error));
        }
    };
    let gate = state.repo_mutation_gate();
    let admission = match gate.admit_mounted_repo(repo_id) {
        Ok(admission) => admission,
        Err(error) => return errors::http(error.server_error()),
    };
    let prepared_external = match state
        .repo
        .prepare_source_control_commit_in_local_repo(&repo_name)
    {
        Ok(prepared) => prepared,
        Err(error) => {
            return errors::http(errors::map_repo_error(errors::ScOp::Commit, error));
        }
    };
    match gate
        .execute_admitted_mounted_repo(admission, &state.tx, || {
            let repo_name = match crate::server::repo_mutation::revalidate_writable_local_repo(
                state, repo_id, &repo_name,
            ) {
                Ok(repo_name) => repo_name,
                Err(error) => {
                    return MutationExecution::not_committed(errors::map_repo_error(
                        errors::ScOp::Commit,
                        error,
                    ));
                }
            };
            match state
                .repo
                .commit_source_control_authority_with_prepared_in_local_repo(
                    &repo_name,
                    &payload.message,
                    prepared_external,
                ) {
                Ok(info) => MutationExecution::committed(
                    (info.clone(), repo_name),
                    MutationPublication::SourceControlCommit {
                        repo_id,
                        branch: None,
                        scope_nonce: None,
                        commit_id: info.id,
                        timestamp: info.timestamp,
                        recovery: MutationPublication::source_control_recovery(repo_id),
                    },
                ),
                Err(deve_core::source_control::CommitAuthorityFailure::NotCommitted(error)) => {
                    MutationExecution::not_committed(errors::map_repo_error(
                        errors::ScOp::Commit,
                        error,
                    ))
                }
                Err(deve_core::source_control::CommitAuthorityFailure::CommittedPartial {
                    external_apply,
                    error,
                }) => MutationExecution::committed_partial(
                    errors::map_repo_error(errors::ScOp::Commit, error),
                    MutationPublication::external_apply_recovery(
                        external_apply.repo_id,
                        external_apply.affected_docs,
                    ),
                ),
            }
        })
        .await
    {
        Ok(MutationExecution::Committed {
            value: (info, repo_name),
            ..
        }) => {
            state
                .repo
                .enqueue_git_mirror_projection_in_local_repo(&repo_name, repo_id, &info);
            Json::<CommitInfo>(info).into_response()
        }
        Ok(MutationExecution::NotCommitted(error)) => errors::http(error),
        Ok(MutationExecution::ProjectionDegraded {
            value: (info, _), ..
        }) => Json::<CommitInfo>(info).into_response(),
        Ok(MutationExecution::CommittedPartial { error, .. }) => errors::http(error),
        Err(error) => errors::http(error.server_error()),
    }
}

pub async fn commit_delegated(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CommitPayload>,
) -> impl IntoResponse {
    let scope_nonce = match super::super::http_scope::require(payload.scope_nonce) {
        Ok(scope_nonce) => scope_nonce,
        Err(error) => return errors::http(error),
    };
    let binding = match super::authorize_http_write(
        &state,
        &payload.repo,
        scope_nonce,
        SourceControlWriteAuthority::DelegatedRemoteProxy,
    ) {
        Ok(binding) => binding,
        Err(error) => return errors::http(error),
    };
    commit_through_gate(&state, binding, payload).await
}

pub async fn commit_plugin_host(
    State(_state): State<Arc<PluginHostState>>,
    Json(payload): Json<CommitPayload>,
) -> impl IntoResponse {
    if let Err(error) = super::super::http_scope::require(payload.scope_nonce) {
        return errors::http(error);
    }
    if let Err(error) = host::ensure_source_control_write_allowed(&payload.repo) {
        return errors::http(error);
    }
    match host::commit_source_control_changes_in_repo(&payload.repo, &payload.message) {
        Ok(info) => Json::<CommitInfo>(info).into_response(),
        Err(e) => errors::http(errors::map_repo_error(errors::ScOp::Commit, e)),
    }
}
