//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/index#internal-path-normalization
//!   - 03_storage/watcher#watcher-contract
//!
//! 复制文档处理器入口。

mod dir_copy;
mod prepare;
mod register;

#[cfg(test)]
mod path_validation_tests;

use super::copy_utils::copy_dir_assets_only;
use super::errors;
use super::{normalize_repo_path_input, resolve_local_write_scope, validate_file_path};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_mutation::MutationExecution;
use crate::server::session::WsSession;
use deve_core::models::NodeKind;
use deve_core::protocol::doc_file_op_errors as path_err;
use prepare::prepare_copy_paths;
use register::{
    CopyRegisterCtx, commit_registration, prepare_registration, prepare_single_file_registration,
};
use std::sync::Arc;

pub async fn handle_copy_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    src_path: String,
    dest_path: String,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(scope) = resolve_local_write_scope(state, ch, session, scope_nonce) else {
        return;
    };

    let Some(src_path) = normalize_repo_path_input(&src_path) else {
        errors::request_failed_scoped(ch, path_err::INVALID_EMPTY_PATH, scope_nonce);
        return;
    };
    let Some(dest_path) = normalize_repo_path_input(&dest_path) else {
        errors::request_failed_scoped(ch, path_err::INVALID_EMPTY_PATH, scope_nonce);
        return;
    };
    if !validate_file_path(&src_path, ch, scope_nonce) {
        return;
    }

    let paths = match prepare_copy_paths(state, ch, &scope, &src_path, &dest_path, scope_nonce) {
        Some(paths) => paths,
        None => return,
    };
    let dst_repo_path = paths.dst_repo_path.as_str();

    let ctx = CopyRegisterCtx {
        state,
        ch,
        scope: &scope,
        scope_nonce,
    };
    let plan = if paths.kind == deve_core::models::NodeKind::Dir {
        prepare_registration(ctx, &paths.src, &src_path, dst_repo_path)
    } else {
        match paths.src_doc_id {
            Some(doc_id) => prepare_single_file_registration(
                ctx,
                src_path.clone(),
                doc_id,
                dst_repo_path.to_string(),
            ),
            None => Err(anyhow::anyhow!("Source doc missing: {src_path}")),
        }
    };
    let plan = match plan {
        Ok(plan) => plan,
        Err(error) => {
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to prepare copied docs: {error}"),
                scope_nonce,
            );
            return;
        }
    };
    let execution = state
        .repo_mutation_gate()
        .execute_repo(scope.repo_id, &state.tx, || {
            let bound_scope = match crate::server::repo_mutation::revalidate_writable_resolved_repo(
                state, &scope,
            ) {
                Ok(scope) => scope,
                Err(error) => return MutationExecution::not_committed(error),
            };
            commit_registration(
                CopyRegisterCtx {
                    state,
                    ch,
                    scope: &bound_scope,
                    scope_nonce,
                },
                &plan,
            )
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { .. }) => {}
        Ok(MutationExecution::NotCommitted(error)) => {
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to register copied docs: {error}"),
                scope_nonce,
            );
            return;
        }
        Ok(MutationExecution::ProjectionDegraded { error, .. })
        | Ok(MutationExecution::CommittedPartial { error, .. }) => {
            errors::storage_persist_failed_scoped(
                ch,
                format!("Copied docs committed partially: {error}"),
                scope_nonce,
            );
            return;
        }
        Err(error) => {
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to serialize copied docs: {error}"),
                scope_nonce,
            );
            return;
        }
    }

    if paths.kind == deve_core::models::NodeKind::Dir
        && !dir_copy::copy_dir(ctx, &paths.src, &paths.dst, &src_path, dst_repo_path)
    {
        return;
    }
    tracing::info!("已复制 {} -> {}", src_path, dst_repo_path);
}

fn copy_dir_on_disk(
    ch: &DualChannel,
    src: &std::path::Path,
    dst: &std::path::Path,
    src_path: &str,
    scope_nonce: Option<u64>,
) -> bool {
    if let Err(e) = copy_dir_assets_only(src, dst) {
        tracing::error!("目录复制失败 {} -> {:?}: {:?}", src_path, dst, e);
        errors::storage_persist_failed_scoped(
            ch,
            format!("Directory copy failed: {}", e),
            scope_nonce,
        );
        return false;
    }
    true
}

pub(super) fn normalize_copy_dest_path(kind: NodeKind, dest_path: &str) -> String {
    if kind == NodeKind::File && !dest_path.ends_with(".md") {
        format!("{}.md", dest_path)
    } else {
        dest_path.to_string()
    }
}
