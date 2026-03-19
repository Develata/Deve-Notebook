use crate::server::session::WsSession;
use deve_core::protocol::{ServerError, ServerErrorCode};

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
    if session.scope_nonce() != requested_scope_nonce {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!(
                "{scope_name} scope nonce is stale: current_scope_nonce={}, requested_scope_nonce={}",
                session.scope_nonce(),
                requested_scope_nonce
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

#[cfg(test)]
mod tests {
    use super::{response_scope_nonce, validate_browser_scope_nonce};
    use crate::server::session::WsSession;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn browser_scope_guard_requires_current_scope_nonce() {
        let mut session = WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(11));
        let missing = validate_browser_scope_nonce(&session, None, "merge").unwrap_err();
        assert_eq!(missing.code, ServerErrorCode::ScRepoContextInvalid);
        let stale = validate_browser_scope_nonce(&session, Some(10), "merge").unwrap_err();
        assert_eq!(stale.code, ServerErrorCode::ScRepoContextInvalid);
        assert!(stale.detail.as_deref().expect("detail").contains("stale"));
        assert!(validate_browser_scope_nonce(&session, Some(11), "merge").is_ok());
    }

    #[test]
    fn non_browser_scope_guard_allows_missing_nonce() {
        let session = WsSession::new();
        assert!(validate_browser_scope_nonce(&session, None, "source control").is_ok());
    }

    #[test]
    fn browser_scope_guard_response_nonce_uses_current_scope_for_missing_requests() {
        let mut session = WsSession::new();
        session.mark_browser_session();
        session.set_scope_nonce(Some(11));
        assert_eq!(response_scope_nonce(&session, None), Some(11));
        assert_eq!(response_scope_nonce(&session, Some(7)), Some(7));
    }
}
