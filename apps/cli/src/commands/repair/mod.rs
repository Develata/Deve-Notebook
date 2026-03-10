mod path_fix;
mod rebuild;
mod restore;
mod shadow;
mod weird_paths;

use anyhow::Result;
use deve_core::ledger::{RepoManager, listing::RepoListing};
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
    repo.set_vault_root(vault_path);
    let repo = Arc::new(repo);
    let sync_manager = SyncManager::new(repo.clone(), vault_path.to_path_buf());

    let repo_names = match target_repo {
        Some(repo_name) => vec![repo_name.to_string()],
        None => repo.list_repos(None)?,
    };
    let fixed_paths = path_fix::repair_repo_prefixed_paths(&repo, &repo_names)?;
    let quarantined_md_dirs = weird_paths::quarantine_md_dirs(&repo, &repo_names)?;
    let restored =
        restore::restore_docs_from_backup(&repo, &sync_manager, backup_root, &repo_names, paths)?;
    let rebuilt_repos = if rebuild_projection {
        rebuild::materialize_repos(&sync_manager, &repo_names)?
    } else {
        0
    };

    println!(
        "repair: quarantined_nil_shadows={} fixed_repo_paths={} quarantined_md_dirs={} restored_docs={} rebuilt_repos={}",
        quarantined, fixed_paths, quarantined_md_dirs, restored, rebuilt_repos
    );
    Ok(())
}
