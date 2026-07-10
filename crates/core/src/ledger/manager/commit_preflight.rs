//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#facts-partition
//!
//! # Source Control Commit Preflight
//!
//! Validates staged commit targets before any ledger facts are appended.

use crate::ledger::RepoManager;
use crate::ledger::manager::commit_plan::CommitTarget;
use crate::source_control::pending_fs;
use crate::utils::path::to_forward_slash;
use anyhow::{Context, Result};

pub(super) fn preflight_staged_commit_targets(
    repo: &RepoManager,
    repo_name: &str,
    targets: &mut [CommitTarget],
) -> Result<()> {
    for target in targets.iter().filter(|target| target.delete_only) {
        preflight_staged_delete_identity(repo, repo_name, target)?;
    }
    for target in targets.iter_mut().filter(|target| !target.delete_only) {
        let disk_path = repo.local_repo_workspace_path(repo_name, &target.path)?;
        let content = std::fs::read_to_string(&disk_path).with_context(|| {
            format!(
                "Failed to read staged workspace file {} at {:?}",
                target.path, disk_path
            )
        })?;
        let current_hash = pending_fs::content_hash(&content);
        if current_hash != target.content_hash {
            anyhow::bail!(
                "staged workspace content changed after stage: {}; rescan and restage before Apply to Ledger",
                target.path
            );
        }
        preflight_staged_upsert_identity(repo, repo_name, target)?;
        target.validated_content = Some(content);
    }
    Ok(())
}

fn preflight_staged_delete_identity(
    repo: &RepoManager,
    repo_name: &str,
    target: &CommitTarget,
) -> Result<()> {
    let Some(doc_id) = target.doc_id else {
        return Ok(());
    };
    if let Some(bound_doc_id) = repo.get_tracked_docid_in_local_repo(repo_name, &target.path)?
        && bound_doc_id != doc_id
    {
        anyhow::bail!(
            "source control delete target path mismatch: staged path {} is bound to {}, but staged doc is {}",
            target.path,
            bound_doc_id,
            doc_id
        );
    }
    if let Some(meta) = repo.get_file_meta_for_doc_in_local_repo(repo_name, doc_id)? {
        let current_path = to_forward_slash(&meta.path);
        if current_path != target.path {
            anyhow::bail!(
                "source control delete target path mismatch: doc {} is at {}, staged path {}",
                doc_id,
                current_path,
                target.path
            );
        }
    }
    Ok(())
}

fn preflight_staged_upsert_identity(
    repo: &RepoManager,
    repo_name: &str,
    target: &CommitTarget,
) -> Result<()> {
    let Some(doc_id) = target.doc_id else {
        if let Some(bound_doc_id) = repo.get_tracked_docid_in_local_repo(repo_name, &target.path)? {
            anyhow::bail!(
                "source control docless upsert target points at tracked path: staged path {} is bound to {}",
                target.path,
                bound_doc_id
            );
        }
        return Ok(());
    };
    if let Some(bound_doc_id) = repo.get_tracked_docid_in_local_repo(repo_name, &target.path)?
        && bound_doc_id != doc_id
    {
        anyhow::bail!(
            "source control upsert target path mismatch: staged path {} is bound to {}, but staged doc is {}",
            target.path,
            bound_doc_id,
            doc_id
        );
    }
    let Some(meta) = repo.get_file_meta_for_doc_in_local_repo(repo_name, doc_id)? else {
        return Ok(());
    };
    let current_path = to_forward_slash(&meta.path);
    if current_path != target.path && !target.has_rename_evidence {
        anyhow::bail!(
            "source control upsert target path mismatch: doc {} is at {}, staged path {} lacks rename evidence",
            doc_id,
            current_path,
            target.path
        );
    }
    Ok(())
}
