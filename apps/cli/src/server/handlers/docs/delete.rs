// apps/cli/src/server/handlers/docs/delete.rs
//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/watcher#watcher-contract
//!   - 03_storage/index#internal-path-normalization
//!
//! # 删除文档处理器

use super::errors;
use super::node_target::resolve_node_target;
use super::{normalize_repo_path_input, resolve_local_write_scope, validate_file_path};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::session::WsSession;
use deve_core::models::NodeKind;
use deve_core::protocol::doc_file_op_errors as path_err;
use std::sync::Arc;

#[cfg(test)]
mod tests;

pub async fn handle_delete_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    path: String,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(scope) = resolve_local_write_scope(state, ch, session, scope_nonce) else {
        return;
    };

    let Some(path) = normalize_repo_path_input(&path) else {
        errors::request_failed_scoped(ch, path_err::INVALID_EMPTY_PATH, scope_nonce);
        return;
    };
    if !validate_file_path(&path, ch, scope_nonce) {
        return;
    }

    tracing::info!("handle_delete_doc: path={}", path);
    let target = match resolve_node_target(state, &scope, &path) {
        Ok(Some(target)) => target,
        Ok(None) => {
            errors::storage_not_found_scoped(
                ch,
                format!("Source not found: {}", path),
                scope_nonce,
            );
            return;
        }
        Err(err) => {
            errors::classified_failure_scoped(ch, err.to_string(), scope_nonce);
            return;
        }
    };

    let execution = state
        .repo_mutation_gate()
        .execute_repo(scope.repo_id, &state.tx, || {
            let scope = match crate::server::repo_mutation::revalidate_writable_resolved_repo(
                state, &scope,
            ) {
                Ok(scope) => scope,
                Err(error) => return MutationExecution::not_committed(error),
            };
            let current_target = match resolve_node_target(state, &scope, &target.repo_path) {
                Ok(Some(current_target))
                    if current_target.kind == target.kind
                        && current_target.doc_id == target.doc_id =>
                {
                    current_target
                }
                Ok(_) => {
                    return MutationExecution::not_committed(anyhow::anyhow!(
                        "Delete target changed while waiting for mutation permit"
                    ));
                }
                Err(error) => return MutationExecution::not_committed(error),
            };
            let documents = current_target.doc_id.map_or(
                deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
                |doc_id| deve_core::protocol::DocumentRecoveryScope::Exact(vec![doc_id]),
            );
            let publication = MutationPublication::document_recovery(scope.repo_id, documents);
            if current_target.kind == NodeKind::Dir {
                if let Err(error) = state.repo.apply_dir_delete_structure_in_local_repo(
                    &scope.repo_name,
                    &current_target.repo_path,
                    "local_delete",
                ) {
                    return MutationExecution::not_committed(error);
                }
                match state
                    .sync_manager
                    .rebuild_projection_local_repo(&scope.repo_name)
                {
                    Ok(_) => MutationExecution::committed((), publication),
                    Err(error) => MutationExecution::projection_degraded((), error, publication),
                }
            } else {
                if let Err(error) = state.repo.apply_file_delete_structure_in_local_repo(
                    &scope.repo_name,
                    &current_target.repo_path,
                    current_target.doc_id,
                    "local_delete",
                ) {
                    return MutationExecution::not_committed(error);
                }
                match state.sync_manager.remove_projection_path_in_local_repo(
                    &scope.repo_name,
                    &current_target.repo_path,
                ) {
                    Ok(_) => MutationExecution::committed((), publication),
                    Err(error) => MutationExecution::projection_degraded((), error, publication),
                }
            }
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { .. }) => {}
        Ok(MutationExecution::NotCommitted(error)) => {
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to delete node: {error}"),
                scope_nonce,
            );
        }
        Ok(MutationExecution::ProjectionDegraded { error, .. })
        | Ok(MutationExecution::CommittedPartial { error, .. }) => {
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to rebuild deleted node projection: {error}"),
                scope_nonce,
            );
        }
        Err(error) => errors::storage_persist_failed_scoped(
            ch,
            format!("Failed to serialize node delete: {error}"),
            scope_nonce,
        ),
    }
}
