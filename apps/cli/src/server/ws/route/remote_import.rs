//! plan_ref:
//!   - 07_network#remote-import-wire-contract
//!
//! Remote Import and Remote Projection push route, separate from Source
//! Control and External Changes.

use crate::server::handlers::{remote_import, remote_projection};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ClientMessage;
use std::sync::Arc;

pub(super) async fn route_remote_import(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    message: ClientMessage,
) {
    if let Some(scope) = message.remote_import_scope_gate()
        && super::scope_guard::reject_invalid_browser_scope_nonce(
            ch,
            session,
            scope.scope_nonce,
            scope.scope_name,
        )
    {
        return;
    }
    match message {
        ClientMessage::RemoteImport(request) => {
            remote_import::handle(state, ch, session, request).await;
        }
        ClientMessage::RemoteProjectionPush(request) => {
            remote_projection::handle_push(state, ch, session, request).await;
        }
        other => super::source_control::route_source_control(state, ch, session, other).await,
    }
}

#[cfg(test)]
mod tests;
