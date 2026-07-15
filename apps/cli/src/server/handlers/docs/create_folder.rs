//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/watcher#watcher-contract
//!
//! 目录创建逻辑。

use super::checked_existing_is_dir;
use super::errors;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::repo_scope::ResolvedRepo;
use crate::server::session::WsSession;
use std::sync::Arc;

pub async fn handle_folder_create(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    path: &std::path::Path,
    filename: &str,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let folder_path = filename.trim_end_matches('/');
    match checked_existing_is_dir(path, "folder create target") {
        Ok(Some(false)) => {
            tracing::error!("目标路径不是目录: {:?}", path);
            errors::storage_conflict_scoped(ch, "Target path is not a directory", scope_nonce);
            return;
        }
        Ok(Some(true)) | Ok(None) => {}
        Err(e) => {
            tracing::error!("检查目录创建目标失败: {:?}", e);
            errors::classified_failure_scoped(
                ch,
                format!("Failed to check folder target: {}", e),
                scope_nonce,
            );
            return;
        }
    }
    let execution = state
        .repo_mutation_gate()
        .execute_repo(scope.repo_id, &state.tx, || {
            let scope =
                match crate::server::repo_mutation::revalidate_writable_resolved_repo(state, scope)
                {
                    Ok(scope) => scope,
                    Err(error) => return MutationExecution::not_committed(error),
                };
            match checked_existing_is_dir(path, "folder create target revalidation") {
                Ok(Some(false)) => {
                    return MutationExecution::not_committed(anyhow::anyhow!(
                        "Target path is not a directory"
                    ));
                }
                Ok(Some(true)) | Ok(None) => {}
                Err(error) => return MutationExecution::not_committed(error),
            }
            let (_node_id, ops) = match state.repo.apply_dir_create_structure_in_local_repo(
                &scope.repo_name,
                folder_path,
                "local_create",
            ) {
                Ok(value) => value,
                Err(error) => return MutationExecution::not_committed(error),
            };
            let publication = MutationPublication::document_recovery(
                scope.repo_id,
                deve_core::protocol::DocumentRecoveryScope::None,
            );
            match state
                .sync_manager
                .rebuild_projection_local_repo(&scope.repo_name)
            {
                Ok(_) => MutationExecution::committed(ops, publication),
                Err(error) => MutationExecution::projection_degraded(ops, error, publication),
            }
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { .. }) => {}
        Ok(MutationExecution::NotCommitted(e)) => {
            tracing::error!("目录结构事实追加失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to create folder: {}", e),
                scope_nonce,
            );
        }
        Ok(MutationExecution::ProjectionDegraded { error, .. })
        | Ok(MutationExecution::CommittedPartial { error, .. }) => {
            tracing::error!("目录创建后投影失败: {:?}", error);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to rebuild created folder projection: {error}"),
                scope_nonce,
            );
        }
        Err(error) => errors::storage_persist_failed_scoped(
            ch,
            format!("Failed to serialize folder create: {error}"),
            scope_nonce,
        ),
    }
}
