//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/watcher#watcher-contract

use super::node_target::resolve_node_target;
use super::{checked_exists, errors};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_mutation::{MountedRepoAdmission, MutationExecution, MutationPublication};
use crate::server::repo_scope::ResolvedRepo;
use crate::server::repo_scope::local_repo_path;
use crate::server::session::WsSession;
use std::sync::Arc;

pub(super) async fn handle_dir_rename(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    admission: MountedRepoAdmission,
    old_path: &str,
    dst_name: &str,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let execution = state
        .repo_mutation_gate()
        .execute_admitted_mounted_repo(admission, &state.tx, || {
            let scope =
                match crate::server::repo_mutation::revalidate_writable_resolved_repo(state, scope)
                {
                    Ok(scope) => scope,
                    Err(error) => return MutationExecution::not_committed(Some(error)),
                };
            match resolve_node_target(state, &scope, old_path) {
                Ok(Some(target))
                    if target.kind == deve_core::models::NodeKind::Dir
                        && target.repo_path == old_path => {}
                Ok(_) => {
                    return MutationExecution::not_committed(Some(anyhow::anyhow!(
                        "Directory rename source changed while waiting for mutation permit"
                    )));
                }
                Err(error) => return MutationExecution::not_committed(Some(error)),
            }
            let destination = match local_repo_path(state, &scope, dst_name) {
                Ok(path) => path,
                Err(error) => return MutationExecution::not_committed(Some(error)),
            };
            match checked_exists(&destination, "directory rename destination revalidation") {
                Ok(false) => {}
                Ok(true) => {
                    return MutationExecution::not_committed(Some(anyhow::anyhow!(
                        "Directory rename destination appeared while waiting for mutation permit"
                    )));
                }
                Err(error) => return MutationExecution::not_committed(Some(error)),
            }
            let ops = match state.repo.apply_dir_rename_structure_in_local_repo(
                &scope.repo_name,
                old_path,
                dst_name,
                "local_rename",
            ) {
                Ok(Some((_node_id, ops))) => ops,
                Ok(None) => return MutationExecution::not_committed(None),
                Err(error) => return MutationExecution::not_committed(Some(error)),
            };
            let publication = MutationPublication::document_recovery(
                scope.repo_id,
                deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
            );
            match state
                .sync_manager
                .rebuild_projection_local_repo(&scope.repo_name)
            {
                Ok(_) => MutationExecution::committed(ops, publication),
                Err(error) => MutationExecution::projection_degraded(ops, Some(error), publication),
            }
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { .. }) => {}
        Ok(MutationExecution::NotCommitted(None)) => {
            errors::storage_not_found_scoped(
                ch,
                format!("Source not tracked: {}", old_path),
                scope_nonce,
            );
        }
        Ok(MutationExecution::NotCommitted(Some(e))) => {
            tracing::error!("目录重命名结构事实失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to rename folder: {}", e),
                scope_nonce,
            );
        }
        Ok(MutationExecution::ProjectionDegraded {
            error: Some(error), ..
        })
        | Ok(MutationExecution::CommittedPartial {
            error: Some(error), ..
        }) => {
            tracing::error!("目录重命名后投影失败: {:?}", error);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to rebuild renamed directory projection: {error}"),
                scope_nonce,
            );
        }
        Ok(MutationExecution::ProjectionDegraded { error: None, .. })
        | Ok(MutationExecution::CommittedPartial { error: None, .. }) => {}
        Err(error) => errors::server_error_scoped(ch, error.server_error(), scope_nonce),
    }
}
