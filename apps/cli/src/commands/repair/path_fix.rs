//! plan_ref:
//!   - 04_storage#internal-path-normalization
//!   - 06_repository#tree-projection-contract
//!   - 12_commands#cli-commands

use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::source_control::pending_fs;
use deve_core::utils::fs::checked_exists;
use std::path::PathBuf;
use std::sync::Arc;

pub(super) fn repair_repo_prefixed_paths(
    repo: &Arc<RepoManager>,
    repo_names: &[String],
) -> Result<usize> {
    let main_repo = repo.local_repo_name().to_string();
    let mut fixed = 0usize;
    for repo_name in repo_names {
        if repo_name == &main_repo {
            continue;
        }
        let docs = repo.list_local_docs(Some(repo_name))?;
        for (doc_id, old_path) in docs {
            let Some(stripped) = old_path
                .strip_prefix(repo_name.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
            else {
                continue;
            };
            if stripped.is_empty() || path_exists_for_repair(repo, repo_name, stripped, doc_id)? {
                continue;
            }
            repair_doc_path(repo, repo_name, doc_id, &old_path, stripped)?;
            println!(
                "repair: normalized repo path {}:{} -> {}",
                repo_name, old_path, stripped
            );
            fixed += 1;
        }
    }
    Ok(fixed)
}

fn repair_doc_path(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    doc_id: deve_core::models::DocId,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    validate_workspace_rename_target(repo, repo_name, old_path, new_path)?;
    let workspace_moved = rename_workspace_file(repo, repo_name, old_path, new_path)?;
    if let Err(err) = repo.repair_rename_doc_mapping_in_local_repo(repo_name, old_path, new_path) {
        rollback_workspace_rename_if_needed(repo, repo_name, new_path, old_path, workspace_moved)?;
        return Err(err);
    }
    if let Err(err) = move_pending(repo, repo_name, doc_id, old_path, new_path) {
        rollback_path_repair(repo, repo_name, old_path, new_path, workspace_moved)?;
        return Err(err);
    }
    Ok(())
}

fn path_exists_for_repair(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    path: &str,
    doc_id: deve_core::models::DocId,
) -> Result<bool> {
    Ok(repo
        .get_tracked_docid_in_local_repo(repo_name, path)?
        .is_some_and(|existing| existing != doc_id))
}

fn validate_workspace_rename_target(
    repo: &RepoManager,
    repo_name: &str,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    let old_abs = repo.local_repo_workspace_path(repo_name, old_path)?;
    if !checked_exists(
        &old_abs,
        &format!(
            "Failed to stat workspace path while validating repair source {}",
            old_path
        ),
    )? {
        return Ok(());
    }
    let new_abs = repo.local_repo_workspace_path(repo_name, new_path)?;
    if checked_exists(
        &new_abs,
        &format!(
            "Failed to stat workspace path while validating repair target {}",
            new_path
        ),
    )? {
        anyhow::bail!(
            "Repair target path already exists in workspace: {}",
            new_path
        );
    }
    Ok(())
}

fn rename_workspace_file(
    repo: &RepoManager,
    repo_name: &str,
    old_path: &str,
    new_path: &str,
) -> Result<bool> {
    let old_abs = repo.local_repo_workspace_path(repo_name, old_path)?;
    if !checked_exists(
        &old_abs,
        &format!(
            "Failed to stat workspace path while renaming repair source {}",
            old_path
        ),
    )? {
        return Ok(false);
    }
    let new_abs = repo.local_repo_workspace_path(repo_name, new_path)?;
    if checked_exists(
        &new_abs,
        &format!(
            "Failed to stat workspace path while renaming repair target {}",
            new_path
        ),
    )? {
        anyhow::bail!(
            "Repair target path already exists in workspace: {}",
            new_path
        );
    }
    if let Some(parent) = new_abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&old_abs, &new_abs)?;
    prune_empty_parents(repo.local_repo_workspace_root(repo_name)?, old_abs.parent());
    Ok(true)
}

fn move_pending(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: deve_core::models::DocId,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    repo.run_on_local_repo(repo_name, |db| {
        let _ = pending_fs::move_for_doc(db, doc_id, old_path, new_path)?;
        Ok(())
    })
}

fn rollback_path_repair(
    repo: &RepoManager,
    repo_name: &str,
    old_path: &str,
    new_path: &str,
    workspace_moved: bool,
) -> Result<()> {
    repo.repair_rename_doc_mapping_in_local_repo(repo_name, new_path, old_path)?;
    rollback_workspace_rename_if_needed(repo, repo_name, new_path, old_path, workspace_moved)
}

fn rollback_workspace_rename_if_needed(
    repo: &RepoManager,
    repo_name: &str,
    moved_path: &str,
    original_path: &str,
    workspace_moved: bool,
) -> Result<()> {
    if !workspace_moved {
        return Ok(());
    }
    let moved_abs = repo.local_repo_workspace_path(repo_name, moved_path)?;
    if !checked_exists(
        &moved_abs,
        &format!(
            "Failed to stat workspace rollback source while restoring {}",
            moved_path
        ),
    )? {
        anyhow::bail!(
            "Repair rollback source path missing in workspace: {}",
            moved_path
        );
    }
    let original_abs = repo.local_repo_workspace_path(repo_name, original_path)?;
    if checked_exists(
        &original_abs,
        &format!(
            "Failed to stat workspace rollback target while restoring {}",
            original_path
        ),
    )? {
        anyhow::bail!(
            "Repair rollback target already exists in workspace: {}",
            original_path
        );
    }
    if let Some(parent) = original_abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&moved_abs, &original_abs)?;
    prune_empty_parents(
        repo.local_repo_workspace_root(repo_name)?,
        moved_abs.parent(),
    );
    Ok(())
}

fn prune_empty_parents(root: PathBuf, start: Option<&std::path::Path>) {
    let mut cursor = start.map(|p| p.to_path_buf());
    while let Some(path) = cursor {
        if path == root || !path.starts_with(&root) {
            break;
        }
        let is_empty = match std::fs::read_dir(&path) {
            Ok(mut iter) => iter.next().is_none(),
            Err(_) => break,
        };
        if !is_empty {
            break;
        }
        let parent = path.parent().map(|p| p.to_path_buf());
        let _ = std::fs::remove_dir(&path);
        cursor = parent;
    }
}

#[cfg(test)]
#[path = "path_fix_rollback_test.rs"]
mod rollback_tests;
#[cfg(test)]
#[path = "path_fix_test.rs"]
mod tests;
