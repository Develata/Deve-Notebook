use crate::server::AppState;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::{ScPathTarget, ServerError};
use std::sync::Arc;

pub(super) fn discard_via_sync_manager(
    state: &Arc<AppState>,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> Result<String, ServerError> {
    let pending = super::service::list_pending(state.repo.as_ref(), selector)?;
    let path = super::service::resolve_path(&pending, target);
    let repo_name = state
        .repo
        .resolve_local_repo_name_for_execution(selector.repo_id, selector.repo_name.as_deref())
        .map_err(super::errors::map_repo_scope_error)?;
    state
        .sync_manager
        .discard_pending_in_local_repo(&repo_name, &path)
        .map_err(|e| {
            super::errors::map_repo_error(super::errors::ScOp::DiscardPending(path.clone()), e)
        })?;
    Ok(path)
}
