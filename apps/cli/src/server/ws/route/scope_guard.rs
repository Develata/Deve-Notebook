//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Browser scope nonce guard for repo-scoped WebSocket routes.

use crate::server::{channel::DualChannel, session::WsSession};
use deve_core::protocol::{ScopeNonce, ServerError, ServerErrorCode};

pub(super) fn validate_browser_scope_nonce(
    session: &WsSession,
    requested_scope_nonce: Option<u64>,
    scope_name: &str,
) -> Result<(), ServerError> {
    if !session.is_browser_session() {
        return Ok(());
    }
    let Some(requested_scope_nonce) = requested_scope_nonce else {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!("{scope_name} scope nonce missing"),
        ));
    };
    let current_scope_nonce = ScopeNonce::new(session.scope_nonce());
    let requested_scope_nonce = ScopeNonce::new(requested_scope_nonce);
    if current_scope_nonce != requested_scope_nonce {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScStaleScope,
            format!(
                "{scope_name} scope nonce is stale: current_scope_nonce={}, requested_scope_nonce={}",
                current_scope_nonce, requested_scope_nonce
            ),
        ));
    }
    Ok(())
}

pub(super) fn response_scope_nonce(
    session: &WsSession,
    requested_scope_nonce: Option<u64>,
) -> Option<u64> {
    if !session.is_browser_session() {
        return requested_scope_nonce;
    }
    requested_scope_nonce.or(Some(session.scope_nonce()))
}

pub(super) fn reject_invalid_browser_scope_nonce(
    ch: &DualChannel,
    session: &WsSession,
    requested_scope_nonce: Option<u64>,
    scope_name: &str,
) -> bool {
    match validate_browser_scope_nonce(session, requested_scope_nonce, scope_name) {
        Ok(()) => false,
        Err(error) => {
            ch.send_protocol_error_with_scope_nonce(
                error,
                response_scope_nonce(session, requested_scope_nonce),
            );
            true
        }
    }
}

#[cfg(test)]
mod tests;
