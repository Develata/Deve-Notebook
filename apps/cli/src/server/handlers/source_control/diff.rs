#[path = "diff_remote.rs"]
mod remote;
#[cfg(test)]
#[path = "diff_remote_test.rs"]
mod remote_test;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ScPathTarget;
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
    target: ScPathTarget,
) {
    if session.is_readonly() {
        remote::handle_remote_diff(state, ch, session, target).await;
        return;
    }
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let selector = super::service::selector_from_scope(&scope);
    let entries = match super::service::list_changes(state.repo.as_ref(), &selector) {
        Ok(entries) => entries,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let normalized = super::service::resolve_path(&entries, &target);
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
