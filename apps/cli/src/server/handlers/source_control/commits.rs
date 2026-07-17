//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
//! Source-control commit handlers.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::session::WsSession;
use deve_core::protocol::{
    MAX_WS_FRAME_BYTES, ScopeNonce, ServerError, ServerErrorCode, ServerMessage,
    server_binary_payload_size,
};
use deve_core::source_control::CommitFileDiffTarget;
use std::sync::Arc;

/// 创建提交 (保存快照)
pub async fn handle_commit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    message: String,
) {
    super::commits_write::commit_with_ack(
        state,
        ch,
        session,
        message,
        "Created commit",
        "Failed to create commit",
    )
    .await;
}

/// 将 External Changes staging 写入 ledger，但不创建 Source Control commit anchor。
pub async fn handle_apply_external_changes(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
) {
    if !session.is_browser_session() {
        return super::errors::send_ws_code_scoped(
            ch,
            ServerErrorCode::ScRepoContextInvalid,
            "External apply requires a browser writer session",
            None,
        );
    }
    let ack_scope_nonce = ScopeNonce::new(session.scope_nonce());
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope =
        match super::repo_scope::resolve_current_authorized_writable_local_repo(state, session) {
            Ok(scope) => scope,
            Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
        };
    let gate = state.repo_mutation_gate();
    let admission = match gate.admit_mounted_repo(scope.repo_id) {
        Ok(admission) => admission,
        Err(error) => return super::errors::send_ws_scoped(ch, error.server_error(), scope_nonce),
    };
    let prepared = match state
        .repo
        .prepare_external_changes_in_local_repo(&scope.repo_name)
    {
        Ok(prepared) => prepared,
        Err(error) => {
            return super::errors::send_ws_scoped(
                ch,
                super::errors::map_repo_error(super::errors::ScOp::ApplyExternalChanges, error),
                scope_nonce,
            );
        }
    };
    let execution = gate
        .execute_admitted_mounted_repo(admission, &state.tx, || {
            let repo_name = match crate::server::repo_mutation::revalidate_writable_local_repo(
                state,
                scope.repo_id,
                &scope.repo_name,
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
                    let publication = MutationPublication::external_apply_recovery(
                        receipt.repo_id,
                        receipt.affected_docs.clone(),
                    );
                    MutationExecution::committed(receipt, publication)
                }
                Err(error) => MutationExecution::not_committed(error),
            }
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { value: receipt, .. }) => {
            tracing::info!("Applied external changes to ledger");
            ch.unicast(ServerMessage::ExternalApplyAck {
                request_id,
                repo_id: scope.repo_id,
                branch: scope.branch.clone(),
                scope_nonce: ack_scope_nonce,
                receipt,
            });
        }
        Ok(MutationExecution::NotCommitted(e)) => {
            tracing::error!("Failed to apply external changes to ledger: {:?}", e);
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
        Ok(MutationExecution::ProjectionDegraded {
            value: receipt,
            error,
            ..
        }) => {
            ch.unicast(ServerMessage::ExternalApplyAck {
                request_id,
                repo_id: scope.repo_id,
                branch: scope.branch.clone(),
                scope_nonce: ack_scope_nonce,
                receipt,
            });
            super::errors::send_ws_scoped(ch, error, scope_nonce);
        }
        Ok(MutationExecution::CommittedPartial { error, .. }) => {
            super::errors::send_ws_scoped(ch, error, scope_nonce);
        }
        Err(error) => super::errors::send_ws_scoped(ch, error.server_error(), scope_nonce),
    }
}

/// 获取提交历史
pub async fn handle_get_commit_history(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    limit: u32,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_repo_scope(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    match super::commits_query::list_commit_history(state, &scope, limit) {
        Ok(commits) => {
            tracing::info!("Returning {} commits", commits.len());
            ch.unicast(ServerMessage::CommitHistory {
                request_id: Some(request_id),
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                commits,
            });
        }
        Err(e) => {
            tracing::error!("Failed to get commit history: {:?}", e);
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
    }
}

/// 获取两个提交之间的差异
pub async fn handle_get_commit_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    commit_a: Option<String>,
    commit_b: String,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_repo_scope(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let query_state = state.clone();
    let query_scope = scope.clone();
    let query_commit_a = commit_a.clone();
    let query_commit_b = commit_b.clone();
    let result = state
        .diff_projection_executor()
        .run_bounded(move || {
            super::commits_query::diff_commit_summaries(
                &query_state,
                &query_scope,
                query_commit_a.as_deref(),
                &query_commit_b,
            )
        })
        .await;
    match result {
        Ok(files) => {
            tracing::info!("Returning diff with {} file changes", files.len());
            let message = ServerMessage::CommitDiffResult {
                request_id: Some(request_id),
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                files,
            };
            match server_binary_payload_size(&message) {
                Ok(size) if size <= MAX_WS_FRAME_BYTES => {
                    let _ = ch.diff_unicast(message).await;
                }
                Ok(size) => super::errors::send_ws_code_scoped(
                    ch,
                    ServerErrorCode::DiffResourceLimit,
                    format!("encoded_bytes={size}; limit={MAX_WS_FRAME_BYTES}"),
                    scope_nonce,
                ),
                Err(_) => super::errors::send_ws_code_scoped(
                    ch,
                    ServerErrorCode::DiffComputeFailed,
                    "commit diff summary serialization failed",
                    scope_nonce,
                ),
            }
        }
        Err(e) => {
            tracing::error!("Failed to get commit diff: {:?}", e);
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
    }
}

pub async fn handle_get_commit_file_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    commit_a: Option<String>,
    commit_b: String,
    target: CommitFileDiffTarget,
) {
    let response_scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_repo_scope(state, session) {
        Ok(scope) => scope,
        Err(error) => return super::errors::send_ws_scoped(ch, error, response_scope_nonce),
    };
    let nonce = ScopeNonce::new(response_scope_nonce.unwrap_or_default());
    let ticket = session.diff_projection_jobs.begin_fixed(
        request_id,
        scope.repo_id,
        scope.branch.clone(),
        nonce,
    );
    let query_state = state.clone();
    let query_scope = scope;
    state.diff_projection_executor().spawn_loaded(
        ticket,
        move || {
            let diff = super::commits_query::diff_commit_file(
                &query_state,
                &query_scope,
                commit_a.as_deref(),
                &commit_b,
                &target,
            )
            .map_err(|error| {
                let detail = error
                    .detail
                    .unwrap_or_else(|| format!("commit diff failed: {:?}", error.code));
                ServerError::with_detail(ServerErrorCode::DiffComputeFailed, detail)
            })?;
            Ok((
                diff.old_content,
                diff.new_content,
                crate::server::diff_projection::DiffJobResponse::Document {
                    doc_id: diff.doc_id,
                    path: diff.path,
                },
            ))
        },
        ch.clone(),
    );
}
