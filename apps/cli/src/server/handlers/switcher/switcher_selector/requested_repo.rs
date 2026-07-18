//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Requested repo selector extraction.

use crate::server::AppState;
use crate::server::shadow_scope;
use anyhow::{Result, anyhow};
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

use super::support::{recover_selector_from_raw_name, select_repo_selector_by_id};

pub(super) fn resolve_requested_repo_name(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    repo_name: &str,
    repo_id: Option<RepoId>,
) -> Result<Option<String>> {
    if let Some(branch) = branch {
        shadow_scope::ensure_remote_branch_available(state, branch)?;
    }
    if let Some(repo_id) = repo_id {
        let Some(selector_by_id) = select_repo_selector_by_id(state, branch, repo_id)? else {
            return Err(anyhow!(
                "Repository UUID not resolved for repository selector {} ({})",
                repo_name,
                repo_id
            ));
        };
        if uuid::Uuid::parse_str(repo_name).is_ok() {
            if repo_name == selector_by_id {
                return Ok(Some(selector_by_id));
            }
            return Err(anyhow!(
                "Session repo mismatch: expected {}, resolved selector {} for exact repository selector {}",
                repo_id,
                selector_by_id,
                repo_name
            ));
        }
        let Some(info) = state
            .repo
            .get_repo_info_for(branch, Some(&selector_by_id))?
        else {
            return Err(anyhow!(
                "Repository UUID not resolved for repository selector {} ({})",
                repo_name,
                repo_id
            ));
        };
        if info.uuid == repo_id && info.name == repo_name {
            return Ok(Some(selector_by_id));
        }
        return Err(anyhow!(
            "Session repo mismatch: expected {}, resolved selector {} for repository label {}",
            repo_id,
            selector_by_id,
            repo_name
        ));
    }
    if uuid::Uuid::parse_str(repo_name).is_ok() {
        return Err(anyhow!("Repository UUID not resolved for {}", repo_name));
    }
    if let Some(exact_selector) = recover_selector_from_raw_name(state, branch, repo_name)? {
        return Ok(Some(exact_selector));
    }
    let repos = state.repo.list_repos(branch)?;
    Ok(repos
        .contains(&repo_name.to_string())
        .then(|| repo_name.to_string()))
}
