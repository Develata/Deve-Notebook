//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Failed branch-switch scope cleanup policy.

use crate::server::repo_scope::should_clear_stale_remote_scope;
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub(super) fn clear_failed_current_scope(session: &mut WsSession, error: &ServerError) {
    if !should_clear_failed_current_scope(session, error) {
        return;
    }
    if session.active_branch.is_some() && shadow_scope::should_clear_missing_remote_branch(error) {
        shadow_scope::clear_stale_remote_branch(session);
        return;
    }
    session.clear_active_repo();
    session.clear_active_db();
    session.clear_sync_binding();
}

fn should_clear_failed_current_scope(session: &WsSession, error: &ServerError) -> bool {
    if session.active_branch.is_some() {
        return match error.code {
            ServerErrorCode::SyncRepoUnbound | ServerErrorCode::ScRepoContextInvalid => {
                should_clear_stale_remote_scope(error)
            }
            _ => false,
        };
    }
    matches!(
        error.code,
        ServerErrorCode::ScRepoContextInvalid | ServerErrorCode::StorageNotFound
    ) || (error.code == ServerErrorCode::SyncRepoUnbound && session.has_runtime_scope_binding())
}
