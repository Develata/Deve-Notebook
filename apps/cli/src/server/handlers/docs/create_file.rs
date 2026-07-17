//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/watcher#watcher-contract
//!
//! 文件创建逻辑。

use super::checked_exists;
use super::errors;
use super::file_register::create_file_from_content;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::repo_scope::ResolvedRepo;
use crate::server::session::WsSession;
use std::sync::Arc;

pub async fn handle_file_create(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    path: &std::path::Path,
    filename: &str,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let gate = state.repo_mutation_gate();
    let admission = match gate.admit_mounted_repo(scope.repo_id) {
        Ok(admission) => admission,
        Err(error) => {
            errors::server_error_scoped(ch, error.server_error(), scope_nonce);
            return;
        }
    };
    match checked_exists(path, "create target") {
        Ok(true) => {
            tracing::error!("目标路径已存在，拒绝从磁盘回填创建: {:?}", path);
            errors::storage_conflict_scoped(
                ch,
                format!("Target already exists: {}", filename),
                scope_nonce,
            );
            return;
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!("检查创建目标失败: {:?}", e);
            errors::classified_failure_scoped(
                ch,
                format!("Failed to check create target: {}", e),
                scope_nonce,
            );
            return;
        }
    }
    match state
        .repo
        .get_tracked_docid_in_local_repo(&scope.repo_name, filename)
    {
        Ok(Some(_)) => {
            errors::storage_conflict_scoped(
                ch,
                format!("Target already tracked: {}", filename),
                scope_nonce,
            );
            return;
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("检查文件跟踪状态失败: {:?}", e);
            errors::classified_failure_scoped(
                ch,
                format!("Failed to check create target: {}", e),
                scope_nonce,
            );
            return;
        }
    }

    let execution = gate
        .execute_admitted_mounted_repo(admission, &state.tx, || {
            let scope =
                match crate::server::repo_mutation::revalidate_writable_resolved_repo(state, scope)
                {
                    Ok(scope) => scope,
                    Err(error) => return MutationExecution::not_committed(error),
                };
            match create_file_from_content(state, &scope, filename, "", "local_create") {
                Ok((doc_id, ops)) => MutationExecution::committed(
                    (doc_id, ops),
                    MutationPublication::document_recovery(
                        scope.repo_id,
                        deve_core::protocol::DocumentRecoveryScope::Exact(vec![doc_id]),
                    ),
                ),
                // The file registration helper spans the authority append and
                // projection writeback. Conservatively publish recovery when
                // it cannot prove the append did not commit.
                Err(error) => MutationExecution::committed_partial(
                    error,
                    MutationPublication::document_recovery(
                        scope.repo_id,
                        deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
                    ),
                ),
            }
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { .. }) => {}
        Ok(MutationExecution::NotCommitted(e))
        | Ok(MutationExecution::CommittedPartial { error: e, .. })
        | Ok(MutationExecution::ProjectionDegraded { error: e, .. }) => {
            tracing::error!("文件创建失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to create file: {}", e),
                scope_nonce,
            );
        }
        Err(error) => errors::server_error_scoped(ch, error.server_error(), scope_nonce),
    }
}
