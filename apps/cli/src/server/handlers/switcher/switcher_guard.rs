//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Switcher scope guard helpers.

use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub(super) fn require_browser_switch_nonce(
    ch: &DualChannel,
    session: &WsSession,
    switch_nonce: Option<u64>,
    action: &str,
) -> bool {
    if !session.is_browser_session() {
        ch.send_protocol_error_with_scope_and_switch_nonce(
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                format!("{action} is only valid for browser sessions"),
            ),
            None,
            switch_nonce,
        );
        return false;
    }
    let Some(switch_nonce) = switch_nonce else {
        ch.send_protocol_error_with_scope_and_switch_nonce(
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                format!("{action} switch nonce missing"),
            ),
            Some(session.scope_nonce()),
            None,
        );
        return false;
    };
    if switch_nonce > session.scope_nonce() {
        return true;
    }
    ch.send_protocol_error_with_scope_and_switch_nonce(
        ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!(
                "{action} switch nonce is stale: current_scope_nonce={}, requested_switch_nonce={}",
                session.scope_nonce(),
                switch_nonce
            ),
        ),
        Some(session.scope_nonce()),
        Some(switch_nonce),
    );
    false
}

#[cfg(test)]
mod tests;
