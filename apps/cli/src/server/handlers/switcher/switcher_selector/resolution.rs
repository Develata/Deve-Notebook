//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Switch target validation result types.

use deve_core::models::{PeerId, RepoId};

pub(super) fn fallback_single_remote_repo(
    target_branch: Option<&PeerId>,
    repos: &[String],
) -> Option<String> {
    (target_branch.is_some() && repos.len() == 1).then(|| repos[0].clone())
}

pub(super) fn unresolved_target_repo_error(
    target_branch: Option<&PeerId>,
    current_repo_name: Option<&str>,
    current_repo_id: Option<RepoId>,
    current_repo_url: Option<&str>,
) -> anyhow::Error {
    let scope = if target_branch.is_some() {
        "Remote"
    } else {
        "Local"
    };
    if let Some(repo_name) = current_repo_name {
        return anyhow::anyhow!("{scope} repository selector not resolved for {}", repo_name);
    }
    if let Some(repo_id) = current_repo_id {
        return anyhow::anyhow!("Repository UUID not resolved for {}", repo_id);
    }
    if let Some(url) = current_repo_url {
        return anyhow::anyhow!("{scope} repository selector not resolved for URL {}", url);
    }
    anyhow::anyhow!("{scope} repository selector not resolved")
}
