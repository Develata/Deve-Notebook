use crate::commands::repo_arg::resolve_local_repo_args;
use anyhow::{Context, Result};
use deve_core::ledger::RepoManager;
use deve_core::ledger::metadata;
use deve_core::sync::rebuild;
use std::path::PathBuf;

/// Recover vault files from ledger data.
///
/// Iterates all documents in each local repo, rebuilds content from
/// ledger ops + snapshots, and writes the result to the vault directory.
pub fn run(
    ledger_dir: &PathBuf,
    vault_path: &PathBuf,
    repo_name: Option<String>,
    snapshot_depth: usize,
) -> Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_names = resolve_local_repo_args(&repo, repo_name.as_deref())?;

    let mut recovered = 0u32;
    let mut skipped = 0u32;

    for rn in &repo_names {
        let docs = repo.run_on_local_repo(rn, |db| metadata::list_docs(db))?;

        for (doc_id, path) in docs {
            if path.is_empty() {
                skipped += 1;
                continue;
            }

            let result = rebuild::rebuild_local_doc_in_repo(&repo, rn, doc_id)
                .with_context(|| format!("Failed to rebuild {}", path))?;

            let target = vault_path.join(rn).join(&path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create dir for {}", path))?;
            }
            std::fs::write(&target, &result.content)
                .with_context(|| format!("Failed to write {}", path))?;
            recovered += 1;
        }
    }

    println!(
        "Recovery complete: {} files recovered, {} skipped",
        recovered, skipped
    );
    Ok(())
}
