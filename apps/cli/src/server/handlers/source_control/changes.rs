use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::ChangeEntry;
use std::sync::Arc;

/// 获取变更列表 (暂存区 + 未暂存)
///
/// 使用 session 上下文确定当前仓库
pub async fn handle_get_changes(state: &Arc<AppState>, ch: &DualChannel, session: &WsSession) {
    // 只读模式没有暂存/未暂存概念，返回空列表
    if session.is_readonly() {
        ch.unicast(ServerMessage::ChangesList {
            staged: vec![],
            unstaged: vec![],
        });
        return;
    }

    let staged = match state.repo.list_staged() {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list staged files: {:?}", e);
            ch.send_error(e.to_string());
            return;
        }
    };

    let unstaged = detect_unstaged_changes(state);

    ch.unicast(ServerMessage::ChangesList { staged, unstaged });
}

/// 检测未暂存的变更
///
/// 从 pending_fs_ops 表读取 Watcher/Scan 检测到的变更，
/// 过滤掉已在暂存区中的路径。
///
/// **Invariant**: pending_fs_ops 是 Working Directory 的单一事实源。
fn detect_unstaged_changes(state: &Arc<AppState>) -> Vec<ChangeEntry> {
    let pending = match state.repo.list_pending_fs() {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list pending fs ops: {:?}", e);
            return Vec::new();
        }
    };

    // 获取已暂存路径集合，用于过滤
    let staged_paths: std::collections::HashSet<String> = state
        .repo
        .list_staged()
        .unwrap_or_default()
        .into_iter()
        .map(|e| deve_core::utils::path::to_forward_slash(&e.path))
        .collect();

    pending
        .into_iter()
        .filter(|e| {
            let normalized = deve_core::utils::path::to_forward_slash(&e.path);
            !staged_paths.contains(&normalized)
        })
        .collect()
}
