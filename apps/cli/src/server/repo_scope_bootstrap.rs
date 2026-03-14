use crate::server::AppState;
use crate::server::session::WsSession;
use anyhow::Result;
use std::sync::Arc;

pub(super) fn fallback_local_repo_name(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<String> {
    if session.scope_nonce() == 0 {
        return Ok(state.repo.local_repo_name().to_string());
    }
    state.repo.resolve_local_repo_name(None, None)
}
