//! plan_ref:
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! # Git Mirror Queue Runtime
//!
//! Detects whether a successful Deve commit should enqueue a Git mirror record.

#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::RepoManager;
#[cfg(not(target_arch = "wasm32"))]
use crate::models::RepoId;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn binding_is_current(
    repo: &RepoManager,
    repo_name: &str,
    expected_repo_id: RepoId,
) -> bool {
    let repo_root = match repo.local_repo_workspace_root(repo_name) {
        Ok(root) => root,
        Err(err) => {
            tracing::warn!(
                repo_name,
                error = %err,
                "Git mirror queue skipped because workspace root is unavailable"
            );
            return false;
        }
    };
    match crate::git_bridge::inspect_repo_root(&repo_root) {
        Ok(status) if status.state == crate::git_bridge::GitMirrorState::Ready => {}
        Ok(_) => return false,
        Err(err) => {
            tracing::warn!(
                repo_name,
                error = %err,
                "Git mirror queue skipped because mirror status could not be inspected"
            );
            return false;
        }
    }
    match repo.run_on_local_repo(repo_name, RepoManager::read_repo_info_from_db) {
        Ok(Some(info)) if info.uuid == expected_repo_id => true,
        Ok(Some(info)) => {
            tracing::warn!(
                repo_name,
                expected_repo_id = %expected_repo_id,
                observed_repo_id = %info.uuid,
                "Git mirror queue skipped because repository identity changed"
            );
            false
        }
        Ok(None) => {
            tracing::warn!(
                repo_name,
                "Git mirror queue skipped because repository metadata is missing"
            );
            false
        }
        Err(err) => {
            tracing::warn!(
                repo_name,
                error = %err,
                "Git mirror queue skipped because repository metadata could not be read"
            );
            false
        }
    }
}
