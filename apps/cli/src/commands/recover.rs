//! plan_ref:
//!   - 03_storage/projection#projection-contract
//!   - 14_commands#cli-commands

use crate::commands::repo_arg::resolve_local_repo_args;
use anyhow::{Context, Result};
use deve_core::ledger::RepoManager;
use deve_core::ledger::metadata;
use deve_core::sync::rebuild;
use std::path::PathBuf;

/// Materialize current documents from ledger data into repo projection workspaces.
///
/// Iterates all documents in each local repo, rebuilds content from
/// ledger ops + snapshots, and writes the result to each repo workspace.
///
/// **Note**: this re-materializes documents that currently exist in the ledger.
/// It does NOT remove stale files (deleted documents, renamed leftovers) from
/// the workspace. For a clean rebuild, clear the target directory first.
pub fn run(ledger_dir: &PathBuf, repo_name: Option<String>, snapshot_depth: usize) -> Result<()> {
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_names = resolve_local_repo_args(&repo, repo_name.as_deref())?;

    let mut recovered = 0u32;
    let mut skipped = 0u32;

    for rn in &repo_names {
        let workspace_root = repo.ensure_local_repo_workspace_identity(rn)?;
        deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&workspace_root)?;
        let docs = repo.run_on_local_repo(rn, metadata::list_docs)?;

        for (doc_id, path) in docs {
            if path.is_empty() {
                skipped += 1;
                continue;
            }

            let result = rebuild::rebuild_local_doc_in_repo(&repo, rn, doc_id)
                .with_context(|| format!("Failed to rebuild {}", path))?;

            let target = repo.local_repo_workspace_path(rn, &path)?;
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

#[cfg(test)]
mod tests {
    use super::run;
    use deve_core::models::{FactActor, Op};

    #[test]
    fn recover_rebuilds_workspace_files_from_ledger() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        let cataloged = crate::test_support::init_cataloged_repo(&ledger_dir, &projection_base, 8)
            .expect("init repo");
        let repo_id = cataloged.repo_id;
        let repo = cataloged.repo;
        let repo_name = repo.local_repo_name().to_string();
        let (doc_id, _) = repo
            .apply_file_structure_in_local_repo(&repo_name, "notes/recovered.md", None, "test")
            .expect("create doc");
        repo.local_fact_writer(FactActor::new("test").expect("actor"))
            .append_content_in_local_repo(
                &repo_name,
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "recovered from ledger".into(),
                },
                1,
            )
            .expect("append content");
        let workspace_path = repo
            .local_repo_workspace_path(&repo_name, "notes/recovered.md")
            .expect("workspace path");
        let workspace_root = repo
            .local_repo_workspace_root(&repo_name)
            .expect("workspace root");
        drop(repo);

        run(&ledger_dir, Some(repo_name.clone()), 8).expect("recover");

        deve_core::utils::notegit::validate_repo_identity_marker(&workspace_root, repo_id)
            .expect("identity marker");
        assert_eq!(
            std::fs::read_to_string(workspace_path).expect("recovered file"),
            "recovered from ledger"
        );
    }
}
