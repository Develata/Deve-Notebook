//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
//! Source-control commit write helper.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::session::WsSession;
use std::sync::Arc;

pub(super) async fn commit_with_ack(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    message: String,
    success_label: &str,
    error_label: &str,
) {
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
    let prepared_external = match state
        .repo
        .prepare_source_control_commit_in_local_repo(&scope.repo_name)
    {
        Ok(prepared) => prepared,
        Err(error) => {
            return super::errors::send_ws_scoped(
                ch,
                super::errors::map_repo_error(super::errors::ScOp::Commit, error),
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
                        super::errors::ScOp::Commit,
                        error,
                    ));
                }
            };
            match state
                .repo
                .commit_source_control_authority_with_prepared_in_local_repo(
                    &repo_name,
                    &message,
                    prepared_external,
                ) {
                Ok(info) => {
                    let publication = MutationPublication::SourceControlCommit {
                        repo_id: scope.repo_id,
                        branch: scope.branch.clone(),
                        scope_nonce,
                        commit_id: info.id.clone(),
                        timestamp: info.timestamp,
                        recovery: MutationPublication::source_control_recovery(scope.repo_id),
                    };
                    MutationExecution::committed(info, publication)
                }
                Err(deve_core::source_control::CommitAuthorityFailure::NotCommitted(error)) => {
                    MutationExecution::not_committed(super::errors::map_repo_error(
                        super::errors::ScOp::Commit,
                        error,
                    ))
                }
                Err(deve_core::source_control::CommitAuthorityFailure::CommittedPartial {
                    external_apply,
                    error,
                }) => MutationExecution::committed_partial(
                    super::errors::map_repo_error(super::errors::ScOp::Commit, error),
                    MutationPublication::external_apply_recovery(
                        external_apply.repo_id,
                        external_apply.affected_docs,
                    ),
                ),
            }
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { value: info, .. }) => {
            tracing::info!("{}: {} - {}", success_label, info.id, info.message);
            state.repo.enqueue_git_mirror_projection_in_local_repo(
                &scope.repo_name,
                scope.repo_id,
                &info,
            );
        }
        Ok(MutationExecution::NotCommitted(e)) => {
            tracing::error!("{}: {:?}", error_label, e);
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
        Ok(MutationExecution::ProjectionDegraded { error, .. }) => {
            super::errors::send_ws_scoped(ch, error, scope_nonce);
        }
        Ok(MutationExecution::CommittedPartial { error, .. }) => {
            tracing::error!(
                "{} after committed External Apply prefix: {:?}",
                error_label,
                error
            );
            super::errors::send_ws_scoped(ch, error, scope_nonce);
        }
        Err(error) => super::errors::send_ws_scoped(ch, error.server_error(), scope_nonce),
    }
}
