//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/index#repo-runtime-layout

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ScPathTarget;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 暂存指定文件
pub async fn handle_stage_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    target: ScPathTarget,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope =
        match super::repo_scope::resolve_current_authorized_writable_local_repo(state, session) {
            Ok(scope) => scope,
            Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
        };
    let selector = super::service::selector_from_scope(&scope);
    match super::service::stage_pending(state.repo.as_ref(), &selector, &target) {
        Ok(path) => {
            tracing::info!("Staged file: {}", path);
            ch.unicast(ServerMessage::StageAck {
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                path,
            });
        }
        Err(e) => {
            tracing::error!("Failed to stage file: {:?}", e);
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
    }
}

/// 取消暂存指定文件
pub async fn handle_unstage_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    target: ScPathTarget,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope =
        match super::repo_scope::resolve_current_authorized_writable_local_repo(state, session) {
            Ok(scope) => scope,
            Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
        };
    let selector = super::service::selector_from_scope(&scope);
    match super::service::unstage_file(state.repo.as_ref(), &selector, &target) {
        Ok(path) => {
            tracing::info!("Unstaged file: {}", path);
            ch.unicast(ServerMessage::UnstageAck {
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                path,
            });
        }
        Err(e) => {
            tracing::error!("Failed to unstage file: {:?}", e);
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
    }
}

/// 批量暂存文件
pub async fn handle_stage_files(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    targets: Vec<ScPathTarget>,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope =
        match super::repo_scope::resolve_current_authorized_writable_local_repo(state, session) {
            Ok(scope) => scope,
            Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
        };
    let selector = super::service::selector_from_scope(&scope);
    match super::service::stage_pending_many(state.repo.as_ref(), &selector, targets) {
        Ok(_) => super::changes::handle_get_changes(state, ch, session, None).await,
        Err(e) => {
            tracing::error!("Failed to stage files: {:?}", e);
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
    }
}

/// 批量取消暂存文件
pub async fn handle_unstage_files(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    targets: Vec<ScPathTarget>,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope =
        match super::repo_scope::resolve_current_authorized_writable_local_repo(state, session) {
            Ok(scope) => scope,
            Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
        };
    let selector = super::service::selector_from_scope(&scope);
    match super::service::unstage_many(state.repo.as_ref(), &selector, targets) {
        Ok(_) => super::changes::handle_get_changes(state, ch, session, None).await,
        Err(e) => {
            tracing::error!("Failed to unstage files: {:?}", e);
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
    }
}
