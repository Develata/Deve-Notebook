use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::run_on_resolved_local_repo;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::ChangeEntry;
use std::sync::Arc;

/// 获取变更列表 (暂存区 + 未暂存)
pub async fn handle_get_changes(state: &Arc<AppState>, ch: &DualChannel, session: &WsSession) {
    if session.is_readonly() {
        ch.unicast(ServerMessage::ChangesList {
            staged: vec![],
            unstaged: vec![],
        });
        return;
    }
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    let staged = match run_on_resolved_local_repo(
        state,
        &scope,
        deve_core::ledger::source_control::list_staged,
    ) {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list staged files: {:?}", e);
            ch.send_error(e.to_string());
            return;
        }
    };
    let unstaged = detect_unstaged_changes(state, &scope);
    ch.unicast(ServerMessage::ChangesList { staged, unstaged });
}

/// 检测未暂存的变更
///
/// **Invariant**: pending_fs_ops 是当前本地 repo 的 Working Directory 单一事实源。
fn detect_unstaged_changes(
    state: &Arc<AppState>,
    scope: &crate::server::repo_scope::ResolvedRepo,
) -> Vec<ChangeEntry> {
    let selector = super::service::selector_from_scope(scope);
    let pending = match super::service::list_pending(state.repo.as_ref(), &selector) {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list pending fs ops: {:?}", e);
            return Vec::new();
        }
    };

    let staged_paths: std::collections::HashSet<String> =
        run_on_resolved_local_repo(state, scope, deve_core::ledger::source_control::list_staged)
            .unwrap_or_default()
            .into_iter()
            .map(|e| deve_core::utils::path::to_forward_slash(&e.path))
            .collect();

    pending
        .into_iter()
        .filter(|e| !staged_paths.contains(&deve_core::utils::path::to_forward_slash(&e.path)))
        .collect()
}
