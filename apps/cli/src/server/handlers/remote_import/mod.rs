//! plan_ref:
//!   - 07_network#remote-import-wire-contract
//!   - 06_backup#remote-import-session-contract
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!
//! Independent Remote Import WebSocket use-case handler.

mod dispatch;
pub(crate) mod http;
mod response;

pub(super) use response::run_blocking;
pub(crate) use response::{RemoteImportPhase, host_error_code};

use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::{RemoteImportRequest, ServerErrorCode};
use std::sync::Arc;

pub async fn handle(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request: RemoteImportRequest,
) {
    let context = request.context().clone();
    let scope =
        match crate::server::repo_scope::resolve_session_repo_or_bootstrap_local(state, session) {
            Ok(scope)
                if scope.repo_id == context.repo_id
                    && scope.branch == context.branch
                    && scope.branch.is_none()
                    && session.scope_nonce() == context.scope_nonce.get() =>
            {
                scope
            }
            Ok(_) => {
                return response::send_error(
                    ch,
                    response::response_context(&context, response::request_identity(&request)),
                    ServerErrorCode::RemoteImportInvalidState,
                );
            }
            Err(error) => {
                tracing::warn!(%error, "Remote Import scope resolution failed");
                return response::send_error(
                    ch,
                    response::response_context(&context, response::request_identity(&request)),
                    ServerErrorCode::RemoteImportInvalidState,
                );
            }
        };

    dispatch::dispatch(state, ch, session, scope, context, request).await;
}
