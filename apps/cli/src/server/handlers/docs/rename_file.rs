//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/watcher#watcher-contract

use super::{checked_exists, errors};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_mutation::{MountedRepoAdmission, MutationExecution, MutationPublication};
use crate::server::repo_scope::ResolvedRepo;
use crate::server::repo_scope::local_repo_path;
use crate::server::session::WsSession;
use std::sync::Arc;

pub(super) async fn handle_file_rename(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    admission: MountedRepoAdmission,
    src_path: &str,
    dst_path: &str,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let doc_id = match state
        .repo
        .get_tracked_docid_in_local_repo(&scope.repo_name, src_path)
    {
        Ok(Some(doc_id)) => doc_id,
        Ok(None) => {
            errors::storage_not_found_scoped(
                ch,
                format!("Document not tracked: {}", src_path),
                scope_nonce,
            );
            return;
        }
        Err(e) => {
            errors::classified_failure_scoped(
                ch,
                format!("Failed to resolve document: {}", e),
                scope_nonce,
            );
            return;
        }
    };
    let execution = state
        .repo_mutation_gate()
        .execute_admitted_mounted_repo(admission, &state.tx, || {
            let scope =
                match crate::server::repo_mutation::revalidate_writable_resolved_repo(state, scope)
                {
                    Ok(scope) => scope,
                    Err(error) => return MutationExecution::not_committed(error),
                };
            match state
                .repo
                .get_tracked_docid_in_local_repo(&scope.repo_name, src_path)
            {
                Ok(Some(current_doc_id)) if current_doc_id == doc_id => {}
                Ok(_) => {
                    return MutationExecution::not_committed(anyhow::anyhow!(
                        "Rename source changed while waiting for mutation permit"
                    ));
                }
                Err(error) => return MutationExecution::not_committed(error),
            }
            let destination = match local_repo_path(state, &scope, dst_path) {
                Ok(path) => path,
                Err(error) => return MutationExecution::not_committed(error),
            };
            match checked_exists(&destination, "rename destination revalidation") {
                Ok(false) => {}
                Ok(true) => {
                    return MutationExecution::not_committed(anyhow::anyhow!(
                        "Rename destination appeared while waiting for mutation permit"
                    ));
                }
                Err(error) => return MutationExecution::not_committed(error),
            }
            let (_doc_id, ops) = match state.repo.apply_file_structure_in_local_repo(
                &scope.repo_name,
                dst_path,
                Some(doc_id),
                "local_rename",
            ) {
                Ok(value) => value,
                Err(error) => return MutationExecution::not_committed(error),
            };
            let publication = MutationPublication::document_recovery(
                scope.repo_id,
                deve_core::protocol::DocumentRecoveryScope::Exact(vec![doc_id]),
            );
            if let Err(error) = state
                .sync_manager
                .persist_doc_in_local_repo(&scope.repo_name, doc_id)
            {
                return MutationExecution::projection_degraded(ops, error, publication);
            }
            if let Err(error) = state
                .sync_manager
                .remove_projection_path_in_local_repo(&scope.repo_name, src_path)
            {
                return MutationExecution::projection_degraded(ops, error, publication);
            }
            MutationExecution::committed(ops, publication)
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { .. }) => {}
        Ok(MutationExecution::NotCommitted(e)) => {
            tracing::error!("重命名结构事实失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to rename file: {}", e),
                scope_nonce,
            );
        }
        Ok(MutationExecution::ProjectionDegraded { error, .. })
        | Ok(MutationExecution::CommittedPartial { error, .. }) => {
            tracing::error!("文件重命名后投影失败: {:?}", error);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to materialize renamed file: {error}"),
                scope_nonce,
            );
        }
        Err(error) => errors::server_error_scoped(ch, error.server_error(), scope_nonce),
    }
}
