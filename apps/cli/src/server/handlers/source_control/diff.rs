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
    session: &mut WsSession,
    request_id: String,
    target: ScPathTarget,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    if session.active_branch.is_some() {
        remote::handle_remote_diff(state, ch, session, request_id, target).await;
        return;
    }
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let (normalized, old_content, new_content) = match state
        .repo
        .workdir_diff_inputs_for_target_in_local_repo(&scope.repo_name, &target)
    {
        Ok(payload) => payload,
        Err(e) => {
            return super::errors::send_ws_scoped(
                ch,
                super::errors::map_repo_error(super::errors::ScOp::DiffDoc(target.path.clone()), e),
                scope_nonce,
            );
        }
    };

    ch.unicast(ServerMessage::DocDiff {
        request_id: Some(request_id),
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        scope_nonce,
        path: normalized,
        old_content,
        new_content,
    });
}
