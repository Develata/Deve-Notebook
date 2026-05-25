//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
//! Remote source-control diff handler.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::source_control::errors;
use crate::server::handlers::source_control::repo_scope;
use crate::server::repo_scope::resolve_local_counterpart_repo;
use crate::server::session::WsSession;
use deve_core::protocol::{ScPathTarget, ServerErrorCode, ServerMessage};
use std::sync::Arc;

use super::remote_content::{local_counterpart_content, resolve_remote_content};

pub(super) async fn handle_remote_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    target: ScPathTarget,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match repo_scope::resolve_current_repo_scope(state, session) {
        Ok(scope) => scope,
        Err(e) => return errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let path = deve_core::utils::path::to_forward_slash(&target.path);
    let (doc_id, new_content) =
        match resolve_remote_content(state, scope.branch.as_ref(), scope.repo_id, &target) {
            Ok(Some(content)) => content,
            Ok(None) => {
                return errors::send_ws_code_scoped(
                    ch,
                    ServerErrorCode::ScDocNotFound,
                    format!("Remote document not found: {}", path),
                    scope_nonce,
                );
            }
            Err(err) => {
                return errors::send_ws_scoped(
                    ch,
                    errors::map_repo_error(errors::ScOp::DiffDoc(path.clone()), err),
                    scope_nonce,
                );
            }
        };

    let local_repo_name = match resolve_local_counterpart_repo(state, &scope) {
        Ok(Some(local_scope)) => local_scope.repo_name,
        Ok(None) => {
            return errors::send_ws_code_scoped(
                ch,
                ServerErrorCode::StorageNotFound,
                "No local repository matched the active remote branch",
                scope_nonce,
            );
        }
        Err(err) => {
            return errors::send_ws_scoped(ch, errors::map_repo_scope_error(err), scope_nonce);
        }
    };
    let old_content = match local_counterpart_content(state.repo.as_ref(), doc_id, &local_repo_name)
    {
        Ok(Some(content)) => content,
        Ok(None) => String::new(),
        Err(err) => {
            return errors::send_ws_scoped(
                ch,
                errors::map_repo_error(errors::ScOp::DiffDoc(path.clone()), err),
                scope_nonce,
            );
        }
    };

    ch.unicast(ServerMessage::DocDiff {
        request_id: Some(request_id),
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        scope_nonce,
        doc_id: Some(doc_id),
        path,
        old_content,
        new_content,
    });
}
