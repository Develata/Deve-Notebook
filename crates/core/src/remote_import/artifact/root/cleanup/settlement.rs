//! plan_ref:
//!   - 03_storage/repair#remote-import-cleanup-repair
//!   - 06_backup#remote-import-removal-owner-plan
//!
//! Identity-exact settlement of a quarantined Remote Import root. The sealed
//! inventory sidecar remains until every payload entry has been deleted so a
//! crash can resume from an exact subset without trusting pathnames.

use super::{
    REMOVAL_PLAN_MAX_BYTES, REMOVAL_PLAN_NAME, RemoteImportArtifactRemovalPlan,
    SealedRemovalInventory, inventory_tree, validate_inventory_binding,
};
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::utils::fs::{HostPathIdentity, HostPathKind, HostQuarantineCut, delete_pinned_identity};
use sha2::{Digest, Sha256};
use std::io::Read;

pub(super) fn delete_quarantined_root(
    plan: &RemoteImportArtifactRemovalPlan,
    cut: &HostQuarantineCut,
) -> RemoteImportResult<()> {
    if let QuarantinedInventoryState::Sealed(inventory) = validate_quarantined_inventory(plan, cut)?
    {
        delete_quarantined_payload(plan, cut, &inventory)?;
    }
    cut.delete()?;
    Ok(())
}

enum QuarantinedInventoryState {
    Sealed(Box<SealedRemovalInventory>),
    EmptyAfterSidecarCut,
}

fn validate_quarantined_inventory(
    plan: &RemoteImportArtifactRemovalPlan,
    cut: &HostQuarantineCut,
) -> RemoteImportResult<QuarantinedInventoryState> {
    if !cut.belongs_to(&plan.root_quarantine) || !cut.is_quarantined_exact()? {
        return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
    }
    let moved_inventory_path = cut.path().join(REMOVAL_PLAN_NAME);
    if !path_entry_exists(&moved_inventory_path)? {
        let actual = inventory_tree(cut.path(), None)?;
        if actual.len() == 1
            && actual[0].relative.as_os_str().is_empty()
            && actual[0].kind == 1
            && actual[0].identity.object_identity() == plan.root.object_identity()
        {
            return Ok(QuarantinedInventoryState::EmptyAfterSidecarCut);
        }
        return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
    }
    let moved_inventory =
        HostPathIdentity::capture(&moved_inventory_path, HostPathKind::RegularFile)?;
    if moved_inventory.object_identity() != plan.inventory.object_identity() {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "quarantined removal inventory identity changed".to_string(),
        ));
    }
    let metadata = std::fs::symlink_metadata(&moved_inventory_path)?;
    if metadata.len() > REMOVAL_PLAN_MAX_BYTES {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "quarantined removal inventory exceeds its read budget".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    crate::utils::fs::open_regular_file_read(
        &moved_inventory_path,
        "quarantined Remote Import removal plan",
    )?
    .take(REMOVAL_PLAN_MAX_BYTES + 1)
    .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > REMOVAL_PLAN_MAX_BYTES
        || format!("{:x}", Sha256::digest(&bytes)) != plan.inventory_digest
        || HostPathIdentity::capture(&moved_inventory_path, HostPathKind::RegularFile)?
            .object_identity()
            != plan.inventory.object_identity()
    {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "quarantined removal inventory content changed".to_string(),
        ));
    }
    let inventory: SealedRemovalInventory = serde_json::from_slice(&bytes)
        .map_err(|error| RemoteImportError::UnsafeArtifactRoot(error.to_string()))?;
    validate_inventory_binding(plan, &inventory)?;
    let actual = inventory_tree(cut.path(), Some(&moved_inventory_path))?;
    if !relocated_inventory_is_exact_subset(&inventory.tree, &actual) {
        return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
    }
    Ok(QuarantinedInventoryState::Sealed(Box::new(inventory)))
}

fn delete_quarantined_payload(
    plan: &RemoteImportArtifactRemovalPlan,
    cut: &HostQuarantineCut,
    inventory: &SealedRemovalInventory,
) -> RemoteImportResult<()> {
    let mut entries = std::fs::read_dir(cut.path())?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    for path in entries {
        if path.file_name().and_then(|name| name.to_str()) == Some(REMOVAL_PLAN_NAME) {
            continue;
        }
        let relative = path
            .strip_prefix(cut.path())
            .map_err(|error| RemoteImportError::UnsafeArtifactRoot(error.to_string()))?;
        let expected = inventory
            .tree
            .iter()
            .find(|entry| entry.relative == relative)
            .ok_or(RemoteImportError::RepoRemovalChanged(plan.repo_id))?;
        let kind = match expected.kind {
            1 => HostPathKind::Directory,
            2 => HostPathKind::RegularFile,
            _ => return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id)),
        };
        let identity = HostPathIdentity::capture(&path, kind)?;
        if identity.object_identity() != expected.identity.object_identity()
            || (kind == HostPathKind::RegularFile
                && std::fs::symlink_metadata(&path)?.len() != expected.len)
        {
            return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
        }
        delete_pinned_identity(&identity)?;
    }
    let sidecar = cut.path().join(REMOVAL_PLAN_NAME);
    let remaining = inventory_tree(cut.path(), Some(&sidecar))?;
    if remaining.len() != 1
        || !remaining[0].relative.as_os_str().is_empty()
        || remaining[0].identity.object_identity() != plan.root.object_identity()
    {
        return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
    }
    Ok(())
}

fn relocated_inventory_is_exact_subset(
    expected: &[super::TreeFingerprint],
    actual: &[super::TreeFingerprint],
) -> bool {
    actual.iter().all(|right| {
        expected.iter().any(|left| {
            left.relative == right.relative
                && left.kind == right.kind
                && left.len == right.len
                && left.identity.object_identity() == right.identity.object_identity()
        })
    })
}

fn path_entry_exists(path: &std::path::Path) -> RemoteImportResult<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
