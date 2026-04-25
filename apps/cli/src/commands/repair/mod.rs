//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 06_repository#tree-projection-contract

mod path_fix;
mod rebuild;
mod restore;
mod shadow;
mod weird_paths;

use crate::commands::repo_arg::resolve_local_repo_args;
use anyhow::{Result, bail};
use deve_core::ledger::RepoManager;
use deve_core::sync::SyncManager;
use std::path::Path;
use std::sync::Arc;

pub fn run(
    ledger_dir: &Path,
    vault_path: &Path,
    backup_root: &Path,
    snapshot_depth: usize,
    target_repo: Option<&str>,
    paths: &[String],
    rebuild_projection: bool,
) -> Result<()> {
    let quarantined = shadow::quarantine_nil_shadow_repos(&ledger_dir.join("remotes"))?;
    let mut repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    repo.set_vault_root_checked(vault_path)?;
    let repo = Arc::new(repo);
    let sync_manager = SyncManager::new_checked(repo.clone(), vault_path.to_path_buf())?;

    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    let fixed_paths = path_fix::repair_repo_prefixed_paths(&repo, &repo_names)?;
    let quarantined_md_dirs = weird_paths::quarantine_md_dirs(&repo, &repo_names)?;
    let restored =
        restore::restore_docs_from_backup(&repo, &sync_manager, backup_root, &repo_names, paths)?;
    let rebuild_report = if rebuild_projection {
        rebuild::rebuild_repos(&sync_manager, &repo_names)?
    } else {
        rebuild::RebuildReport::default()
    };

    println!(
        "repair: quarantined_nil_shadows={} fixed_repo_paths={} quarantined_md_dirs={} restored_docs={} rebuilt_repos={} authority_corrupt_repos={}",
        quarantined,
        fixed_paths,
        quarantined_md_dirs,
        restored,
        rebuild_report.rebuilt,
        rebuild_report.authority_corrupt
    );
    if rebuild_report.authority_corrupt > 0 {
        bail!(
            "repair: {} repo(s) have corrupted Structure Facts authority; projection rebuild skipped",
            rebuild_report.authority_corrupt
        );
    }
    Ok(())
}
