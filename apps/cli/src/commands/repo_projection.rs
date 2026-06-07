//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 03_storage/projection#projection-locator-contract

use super::repo_arg::resolve_local_repo_arg;
use anyhow::Result;
use deve_core::ledger::RepoManager;
use std::path::Path;
use std::sync::Arc;

pub fn set(
    ledger_dir: &Path,
    repo_selector: &str,
    base: &Path,
    snapshot_depth: usize,
) -> Result<()> {
    let repo = Arc::new(RepoManager::init(ledger_dir, snapshot_depth, None, None)?);
    let repo_name = resolve_local_repo_arg(&repo, Some(repo_selector))?;
    let locator = repo.set_projection_base_for_local_repo(&repo_name, base)?;
    deve_core::sync::SyncManager::new_checked(repo.clone())?.materialize_local_repo(&repo_name)?;
    let workspace = repo.check_projection_locator_for_local_repo(&repo_name)?;
    println!(
        "Projection Locator set: {} {} -> {:?}",
        repo_name, locator.repo_id, workspace
    );
    Ok(())
}

pub fn list(ledger_dir: &Path, snapshot_depth: usize) -> Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    for locator in repo.list_projection_locators()? {
        println!(
            "{} {} {:?}",
            locator.repo_id, locator.repo_name_hint, locator.projection_base_abs
        );
    }
    Ok(())
}

pub fn check(ledger_dir: &Path, repo_selector: &str, snapshot_depth: usize) -> Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_name = resolve_local_repo_arg(&repo, Some(repo_selector))?;
    let workspace = repo.check_projection_locator_for_local_repo(&repo_name)?;
    println!("Projection workspace OK: {} -> {:?}", repo_name, workspace);
    Ok(())
}

pub fn drift(
    ledger_dir: &Path,
    repo_selector: &str,
    root: Option<&Path>,
    snapshot_depth: usize,
) -> Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_name = resolve_local_repo_arg(&repo, Some(repo_selector))?;
    let report = match root {
        Some(root) => deve_core::sync::drift_detect::detect_repo_drift_at_workspace_root(
            &repo, &repo_name, root,
        )?,
        None => deve_core::sync::drift_detect::detect_repo_drift(&repo, &repo_name)?,
    };
    println!(
        "projection_drift[{}]: unexplained={} explained={}",
        repo_name,
        report.unexplained.len(),
        report.explained_count
    );
    for entry in report.unexplained {
        println!("  {:?} {}", entry.kind, entry.path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{check, drift, list, set};
    use crate::commands::init;
    use deve_core::ledger::RepoManager;
    use deve_core::sync::SyncManager;
    use std::sync::Arc;

    #[test]
    fn projection_locator_init_writes_locator_without_vault_path_config() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        let ledger = root.join("ledger");
        let notes = root.join("notes");

        init::run(
            &ledger,
            "default",
            &notes,
            root.to_path_buf(),
            8,
            None,
            None,
        )?;

        let config = std::fs::read_to_string(root.join("config.toml"))?;
        assert!(!config.contains("vault_path"));
        assert!(root.join("ledger/.host/projection-locators.toml").is_file());
        let mut workspaces = std::fs::read_dir(&notes)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        workspaces.sort();
        assert_eq!(workspaces.len(), 1);
        assert!(
            workspaces[0]
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("default--"))
        );
        assert!(workspaces[0].join(".notegit").is_dir());
        Ok(())
    }

    #[test]
    fn projection_locator_set_list_check_roundtrip() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        let repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
        repo.set_projection_base_for_local_repo("default", &first)?;

        set(&ledger, "default", &second, 8)?;
        list(&ledger, 8)?;
        check(&ledger, "default", 8)?;

        let reopened = RepoManager::init(&ledger, 8, None, None)?;
        let workspace = reopened.local_repo_workspace_root("default")?;
        assert_eq!(
            workspace,
            std::fs::canonicalize(&second)?.join(format!(
                "default--{}",
                reopened.get_repo_info()?.expect("default repo").uuid
            ))
        );
        assert!(workspace.join(".notegit").is_dir());
        assert!(workspace.join(".gitignore").is_file());
        Ok(())
    }

    #[test]
    fn projection_locator_check_fails_when_workspace_root_is_missing() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let base = dir.path().join("notes");
        let repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
        repo.set_projection_base_for_local_repo("default", &base)?;

        let err = check(&ledger, "default", 8)
            .expect_err("projection check must verify workspace root exists");
        assert!(
            err.to_string()
                .contains("Failed to canonicalize Projection workspace root"),
            "{err}"
        );
        Ok(())
    }

    #[test]
    fn projection_drift_reports_unexplained_workspace_changes() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let base = dir.path().join("notes");
        let repo = Arc::new(RepoManager::init(
            &ledger,
            8,
            Some("default"),
            Some("urn:default"),
        )?);
        repo.set_projection_base_for_local_repo("default", &base)?;
        SyncManager::new_checked(repo.clone())?.materialize_local_repo("default")?;
        let root = repo.local_repo_workspace_root("default")?;
        std::fs::write(root.join("extra.md"), "extra\n")?;

        let report = deve_core::sync::drift_detect::detect_repo_drift_at_workspace_root(
            repo.as_ref(),
            "default",
            &root,
        )?;
        assert_eq!(report.unexplained.len(), 1);
        assert_eq!(report.unexplained[0].path, "extra.md");

        drift(&ledger, "default", Some(&root), 8)?;
        Ok(())
    }
}
