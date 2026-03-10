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
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let staged = match run_on_resolved_local_repo(
        state,
        &scope,
        deve_core::ledger::source_control::list_staged,
    ) {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list staged files: {:?}", e);
            return super::errors::send_ws(
                ch,
                super::errors::map_repo_error(super::errors::ScOp::ListChanges, e),
            );
        }
    };
    let unstaged = match detect_unstaged_changes(state, &scope) {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list unstaged files: {:?}", e);
            return super::errors::send_ws(ch, e);
        }
    };
    ch.unicast(ServerMessage::ChangesList { staged, unstaged });
}

/// 检测未暂存的变更
///
/// **Invariant**: pending_fs_ops 是当前本地 repo 的 Working Directory 单一事实源。
fn detect_unstaged_changes(
    state: &Arc<AppState>,
    scope: &crate::server::repo_scope::ResolvedRepo,
) -> super::service::ScResult<Vec<ChangeEntry>> {
    let selector = super::service::selector_from_scope(scope);
    let pending = super::service::list_pending(state.repo.as_ref(), &selector)?;

    let staged_paths: std::collections::HashSet<String> =
        run_on_resolved_local_repo(state, scope, deve_core::ledger::source_control::list_staged)
            .map_err(|e| super::errors::map_repo_error(super::errors::ScOp::ListChanges, e))?
            .into_iter()
            .map(|e| deve_core::utils::path::to_forward_slash(&e.path))
            .collect();

    Ok(pending
        .into_iter()
        .filter(|e| !staged_paths.contains(&deve_core::utils::path::to_forward_slash(&e.path)))
        .collect())
}
