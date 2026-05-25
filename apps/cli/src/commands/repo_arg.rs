//! plan_ref:
//!   - 04_repository#repo-selector-resolution-contract
//!   - 14_commands#cli-commands

use anyhow::Result;
use deve_core::ledger::RepoManager;

pub(crate) fn resolve_local_repo_arg(repo: &RepoManager, selector: Option<&str>) -> Result<String> {
    let repo_id = selector.and_then(|value| uuid::Uuid::parse_str(value).ok());
    let repo_name = if repo_id.is_some() { None } else { selector };
    repo.resolve_local_repo_name_for_execution(repo_id, repo_name)
}

pub(crate) fn resolve_local_repo_args(
    repo: &RepoManager,
    selector: Option<&str>,
) -> Result<Vec<String>> {
    match selector {
        Some(selector) => Ok(vec![resolve_local_repo_arg(repo, Some(selector))?]),
        None => repo.list_local_repo_names_for_execution(),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_local_repo_arg;
    use deve_core::ledger::RepoManager;
    use tempfile::TempDir;

    #[test]
    fn resolves_local_repo_uuid_argument() {
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let repo = RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default"))
            .expect("init default");
        let repo_id = repo
            .get_repo_info()
            .expect("repo info")
            .expect("default present")
            .uuid;

        assert_eq!(
            resolve_local_repo_arg(&repo, Some(&repo_id.to_string())).expect("resolve uuid"),
            "default"
        );
    }
}
