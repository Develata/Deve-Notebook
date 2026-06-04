//! plan_ref:
//!   - 04_repository#repo-catalog-repair-contract
//!   - 14_commands#cli-commands

use super::{RepairOptions, run};
use deve_core::ledger::RepoManager;
use tempfile::TempDir;

#[test]
fn repair_run_realigns_legacy_workspace_before_workspace_steps() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("vault");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default"))?;
    let repo_id = repo.get_repo_info()?.expect("default repo metadata").uuid;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    std::fs::create_dir_all(projection_base.join("default").join(".notegit"))?;
    drop(repo);

    run(
        &ledger_dir,
        8,
        RepairOptions {
            backup_root: &dir.path().join("backups"),
            target_repo: Some("default"),
            paths: &[],
            rebuild_projection: false,
            check: false,
        },
    )?;

    let repaired_root = projection_base.join(format!("default--{repo_id}"));
    deve_core::utils::notegit::validate_repo_identity_marker(&repaired_root, repo_id)?;
    assert!(!projection_base.join("default").exists());
    Ok(())
}
