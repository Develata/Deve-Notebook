#[path = "diff_remote.rs"]
mod remote;

use crate::server::AppState;
use crate::server::channel::DualChannel;
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
    let normalized = deve_core::utils::path::to_forward_slash(&path);
    let (old_content, new_content) = match state
        .repo
        .workdir_diff_inputs_in_local_repo(&scope.repo_name, &normalized)
    {
        Ok(payload) => payload,
        Err(e) => {
            return super::errors::send_ws(
                ch,
                super::errors::map_repo_error(super::errors::ScOp::DiffDoc(normalized.clone()), e),
            );
        }
    };

    ch.unicast(ServerMessage::DocDiff {
        path: normalized,
        old_content,
        new_content,
    });
}
