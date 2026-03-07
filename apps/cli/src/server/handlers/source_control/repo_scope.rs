use crate::server::repo_scope::{ResolvedRepo, resolve_session_repo};
use crate::server::{AppState, session::WsSession};
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub fn resolve_current_local_repo(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<ResolvedRepo> {
    let scope = resolve_session_repo(state, session)?;
    if scope.branch.is_some() {
        return Err(anyhow!(
            "Source control local op requested on remote branch: {}",
            scope.repo_name
        ));
    }
    Ok(scope)
}
