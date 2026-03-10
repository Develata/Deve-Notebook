#[path = "diff_remote.rs"]
mod remote;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::run_on_resolved_local_repo;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 获取文档的 Diff。
///
/// **Local 分支**: 已提交版本 (左) vs 当前版本 (右)
/// **Remote 分支**: Local 对应文档 (左) vs Remote 文档 (右)
pub async fn handle_get_doc_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    if session.is_readonly() {
        remote::handle_remote_diff(state, ch, session, path).await;
        return;
    }
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
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
        Err(e) => {
            return super::errors::send_ws(
                ch,
                super::errors::map_repo_error(super::errors::ScOp::DiffDoc(path.clone()), e),
            );
        }
    };

    ch.unicast(ServerMessage::DocDiff {
        path,
        old_content,
        new_content,
    });
}
