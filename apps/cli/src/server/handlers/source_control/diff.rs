use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{resolve_session_repo, run_on_resolved_local_repo};
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 获取文档的 Diff
///
/// **Local 分支**: 已提交版本 (左) vs 当前版本 (右)
/// **Remote 分支**: Local 对应文档 (左) vs Remote 文档 (右)
pub async fn handle_get_doc_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    // 只读模式 (Remote 分支) → 跨分支 diff
    if session.is_readonly() {
        handle_remote_diff(state, ch, session, path).await;
        return;
    }
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    let (old_content, new_content) = match run_on_resolved_local_repo(state, &scope, |db| {
        let doc_id = deve_core::ledger::metadata::get_docid(db, &path)?
            .ok_or_else(|| anyhow::anyhow!("Document not found: {}", path))?;
        let old_content = deve_core::source_control::changes::get_committed_content(db, doc_id)?
            .unwrap_or_default();
        let ops = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
        let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
        Ok((old_content, deve_core::state::reconstruct_content(&entries)))
    }) {
        Ok(payload) => payload,
        Err(e) => return ch.send_error(e.to_string()),
    };

    ch.unicast(ServerMessage::DocDiff {
        path,
        old_content,
        new_content,
    });
}

/// Remote 分支的跨分支 Diff
///
/// **左侧 (old)**: Local 分支对应文档 (需匹配 Repo URL)
/// **右侧 (new)**: Remote 分支文档 (当前只读视图)
async fn handle_remote_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    // 从 Remote DB 获取文档内容 (右侧)
    let new_content = match get_remote_doc_content(session, &path) {
        Some(content) => content,
        None => {
            ch.send_error(format!("Remote document not found: {}", path));
            return;
        }
    };

    // 1. 获取 Remote Repo URL
    let remote_url = match state
        .repo
        .get_repo_url(session.active_branch.as_ref(), &scope.repo_name)
    {
        Ok(url) => url,
        Err(e) => {
            tracing::error!("Failed to get remote repo URL: {:?}", e);
            None
        }
    };

    // 2. 查找匹配的 Local Repo (通过 URL)
    let local_repo_name = if let Some(url) = remote_url {
        state.repo.find_local_repo_name_by_url(&url).unwrap_or(None)
    } else {
        None
    };

    if local_repo_name.is_none() {
        tracing::warn!("No matching local repo found for remote URL, treating as new file.");
    }

    // 从 Local 分支获取对应文档 (左侧)
    let old_content = get_local_counterpart(state, &path, local_repo_name);

    ch.unicast(ServerMessage::DocDiff {
        path,
        old_content,
        new_content,
    });
}

/// 从 Remote DB 读取文档内容
fn get_remote_doc_content(session: &WsSession, path: &str) -> Option<String> {
    let db = session.get_active_db()?;
    let doc_id = deve_core::ledger::metadata::get_docid(&db.db, path).ok()??;
    let ops = deve_core::ledger::ops::get_ops_from_db(&db.db, doc_id).ok()?;
    let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
    Some(deve_core::state::reconstruct_content(&entries))
}

/// 从 Local 分支读取对应路径的文档
///
/// * `repo_name`: 指定的本地仓库名称 (若 None 则返回空，表示无对应仓库)
fn get_local_counterpart(state: &Arc<AppState>, path: &str, repo_name: Option<String>) -> String {
    if let Some(name) = repo_name {
        state
            .repo
            .run_on_local_repo(&name, |db| {
                let doc_id = match deve_core::ledger::metadata::get_docid(db, path) {
                    Ok(Some(id)) => id,
                    Ok(None) => return Ok(None),
                    Err(e) => return Err(e),
                };

                let ops = match deve_core::ledger::ops::get_ops_from_db(db, doc_id) {
                    Ok(ops) => ops,
                    Err(e) => return Err(e),
                };

                let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
                Ok(Some(deve_core::state::reconstruct_content(&entries)))
            })
            .unwrap_or(None)
            .unwrap_or_default()
    } else {
        // 无对应本地仓库 -> 视为远端新增文件
        String::new()
    }
}
