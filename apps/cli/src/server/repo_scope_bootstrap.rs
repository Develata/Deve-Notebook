//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Local repo bootstrap fallback for scoped sessions.

use crate::server::AppState;
use crate::server::session::WsSession;
use anyhow::Result;
use std::sync::Arc;

pub(super) fn fallback_local_repo_name(
    state: &Arc<AppState>,
    _session: &WsSession,
) -> Result<String> {
    state.repo.resolve_local_repo_name_for_execution(None, None)
}
