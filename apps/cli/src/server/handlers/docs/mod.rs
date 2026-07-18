// apps/cli/src/server/handlers/docs/mod.rs
//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#tree-projection-contract
//!
//! # 文档 CRUD 处理器模块
//!
//! 将文档操作拆分为独立子模块，提高可维护性。
//!
//! ## 子模块
//! - `create`: 创建文档
//! - `rename`: 重命名/移动文档
//! - `delete`: 删除文档
//! - `copy`: 复制文档

mod copy;
mod copy_utils;
mod create;
mod create_file;
mod create_folder;
mod delete;
mod errors;
mod file_register;
mod node_target;
mod path_validation;
mod rename;
mod rename_dir;
mod rename_file;

pub use copy::handle_copy_doc;
pub use create::handle_create_doc;
pub use delete::handle_delete_doc;
pub use path_validation::{normalize_repo_path_input, validate_file_path, validate_folder_path};
pub use rename::{handle_move_doc, handle_rename_doc};

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{
    ResolvedRepo, ensure_resolved_local_repo_writable, map_repo_scope_error,
    resolve_session_repo_or_bootstrap_local,
};
use crate::server::session::WsSession;
use anyhow::Context;
use deve_core::models::RepoId;
use deve_core::protocol::ServerMessage;
pub(super) use deve_core::utils::fs::checked_exists;
use std::path::Path;
use std::sync::Arc;

pub fn notify_fs_refresh(ch: &DualChannel, repo_id: RepoId, path: &str, change_type: &str) {
    ch.broadcast(ServerMessage::FsChangeDetected {
        repo_id: Some(repo_id),
        branch: None,
        scope_nonce: None,
        path: path.to_string(),
        change_type: change_type.to_string(),
        has_conflict: false,
    });
}

pub(super) fn resolve_local_write_scope(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
) -> Option<ResolvedRepo> {
    let scope = match resolve_session_repo_or_bootstrap_local(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(err), scope_nonce);
            return None;
        }
    };
    if scope.branch.is_none()
        && (session.active_repo.as_deref() != Some(scope.session_name.as_str())
            || session.active_repo_id != Some(scope.repo_id))
    {
        session.switch_repo(scope.session_name.clone(), Some(scope.repo_id));
    }
    if scope.branch.is_some() {
        tracing::debug!("Docs write rejected: resolved scope is readonly (remote branch)");
        errors::remote_branch_readonly_scoped(ch, scope_nonce);
        return None;
    }
    if let Err(error) = ensure_resolved_local_repo_writable(state, &scope) {
        tracing::debug!(
            repo_name = %scope.repo_name,
            "Docs write rejected: local repo projection is degraded"
        );
        ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
        return None;
    }
    Some(scope)
}

pub(super) fn checked_existing_is_dir(path: &Path, context: &str) -> anyhow::Result<Option<bool>> {
    if !checked_exists(path, context)? {
        return Ok(None);
    }
    Ok(Some(
        std::fs::metadata(path)
            .with_context(|| {
                format!(
                    "Failed to read metadata for {}: {}",
                    context,
                    path.display()
                )
            })?
            .is_dir(),
    ))
}
