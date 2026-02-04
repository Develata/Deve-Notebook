use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::RepoType;
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

    let unstaged = detect_unstaged_changes(state, session);

    ch.unicast(ServerMessage::ChangesList { staged, unstaged });
}

/// 检测未暂存的变更
///
/// 使用 session 的 active_db 或回退到 active_repo
fn detect_unstaged_changes(state: &Arc<AppState>, session: &WsSession) -> Vec<ChangeEntry> {
    let mut changes = Vec::new();

    // 使用 session 锁定的数据库，或回退到默认逻辑
    let docs = if let Some(handle) = session.get_active_db() {
        deve_core::ledger::metadata::list_docs(&handle.db)
    } else {
        let repo_id = crate::server::handlers::get_repo_id(state);
        state.repo.list_docs(&RepoType::Local(repo_id))
    };

    let docs = match docs {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("Failed to list docs: {:?}", e);
            return changes;
        }
    };

    let staged_paths: std::collections::HashSet<String> = state
        .repo
        .list_staged()
        .unwrap_or_default()
        .into_iter()
        .map(|e| deve_core::utils::path::to_forward_slash(&e.path))
        .collect();

    for (doc_id, path) in docs {
        let normalized = deve_core::utils::path::to_forward_slash(&path);
        if staged_paths.contains(&normalized) {
            continue;
        }

        let current = match state.repo.get_local_ops(doc_id) {
            Ok(ops) => {
                let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
                Some(deve_core::state::reconstruct_content(&entries))
            }
            Err(e) => {
                tracing::error!("Failed to get local ops for {}: {:?}", path, e);
                // On error, we shouldn't treat it as "empty/deleted", better to skip detecting change for this file
                // to avoid false positives.
                continue;
            }
        };

        let committed = state.repo.get_committed_content(doc_id).ok().flatten();

        if let Some(status) = state
            .repo
            .detect_change(committed.as_deref(), current.as_deref())
        {
            changes.push(ChangeEntry { path, status });
        }
    }

    changes
}
