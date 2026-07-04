//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#command-palette-shortcuts
//!
//! WebSocket admission for remote Markdown projection transport intents.

use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ServerErrorCode;
use deve_core::protocol::{
    REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL, RemoteProjectionDirection,
    RemoteProjectionProvider,
};
use std::sync::Arc;

pub async fn handle_remote_projection_transport(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_writable_local_repo(state, session) {
        Ok(scope) => scope,
        Err(error) => return super::errors::send_ws_scoped(ch, error, scope_nonce),
    };
    if let Err(error) = state
        .repo
        .validate_local_repo_workspace_identity(&scope.repo_name)
    {
        return super::errors::send_ws_scoped(
            ch,
            super::errors::map_repo_error(super::errors::ScOp::RemoteProjection, error),
            scope_nonce,
        );
    }

    tracing::info!(
        provider = provider.as_str(),
        direction = direction.as_str(),
        repo = scope.repo_name,
        "remote projection provider I/O is not wired"
    );
    super::errors::send_ws_code_scoped(
        ch,
        ServerErrorCode::ScRepoContextInvalid,
        REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL,
        scope_nonce,
    );
}
