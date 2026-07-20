//! plan_ref:
//!   - 04_repository#repo-catalog-repair-contract
//!   - 14_commands#cli-commands

use super::{RepairOptions, run};
use tempfile::TempDir;

// Under the new repository contract, projection workspace segments are the bare
// canonical RepoId and are IMMUTABLE, fixed at repo-creation time by the prepared
// projection locator. A cataloged repo therefore always has its workspace at the
// canonical `<base>/<repo_id>` location, so the legacy "workspace directory
// realignment" step the original test exercised (moving `<base>/default` ->
// `<base>/default--<uuid>`) no longer exists in `repair::run`. This test now
// verifies the surviving invariant: `repair::run` completes end-to-end against a
// cataloged repo (resolved by its canonical RepoId) and does NOT disturb the
// canonical workspace identity marker while running the workspace steps.
#[test]
fn repair_run_realigns_legacy_workspace_before_workspace_steps() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("vault");
    let cataloged = crate::test_support::init_cataloged_repo(&ledger_dir, &projection_base, 8)?;
    let repo_id = cataloged.repo_id;
    let repo_name = repo_id.to_string();
    let canonical_root = cataloged.workspace_root.clone();
    drop(cataloged);

    run(
        &ledger_dir,
        8,
        RepairOptions {
            backup_root: &dir.path().join("backups"),
            target_repo: Some(&repo_name),
            paths: &[],
            rebuild_projection: false,
            check: false,
        },
    )?;

    deve_core::utils::notegit::validate_repo_identity_marker(&canonical_root, repo_id)?;
    Ok(())
}
