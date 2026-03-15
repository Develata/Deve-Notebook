use crate::server::AppState;
use anyhow::Result;
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

pub(super) fn recover_remote_repo_name_from_selector(
    state: &Arc<AppState>,
    branch: &PeerId,
    repo_name: &str,
    expected_repo_id: Option<RepoId>,
) -> Result<Option<String>> {
    let resolved = state.repo.find_remote_repo_selector(branch, repo_name)?;
    if resolved.as_deref() == Some(repo_name) {
        return Ok(resolved);
    }
    if let Ok(raw_uuid) = uuid::Uuid::parse_str(repo_name) {
        if expected_repo_id == Some(raw_uuid) {
            return state.repo.find_remote_repo_selector_by_id(branch, raw_uuid);
        }
        if has_remote_display_name(state, branch, repo_name)? {
            tracing::warn!(
                "Refusing to recover UUID-shaped remote selector without bound UUID because matching display name exists: branch={}, raw_name={}, resolved_selector={:?}",
                branch,
                repo_name,
                resolved
            );
            return Ok(None);
        }
        return state.repo.find_remote_repo_selector_by_id(branch, raw_uuid);
    }
    tracing::warn!(
        "Refusing to recover remote repo selector from display-only name without UUID: branch={}, raw_name={}, resolved_selector={:?}",
        branch,
        repo_name,
        resolved
    );
    Ok(None)
}

fn has_remote_display_name(state: &Arc<AppState>, branch: &PeerId, raw_name: &str) -> Result<bool> {
    state.repo.has_remote_display_name(branch, raw_name)
}
