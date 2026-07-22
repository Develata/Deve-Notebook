//! plan_ref:
//!   - 03_storage/repair#remote-import-cleanup-repair
//!   - 06_backup#remote-import-removal-owner-plan

mod inventory;
mod settlement;

use self::inventory::{TreeFingerprint, inventory_digest, inventory_tree};
use super::{RemoteImportArtifactRoot, require_direct_child};
use crate::models::RepoId;
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::types::RemoteImportSessionId;
use crate::utils::fs::{
    HostPathIdentity, HostPathKind, HostPathState, HostQuarantineCut, HostQuarantinePlan,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;

const REMOVAL_PLAN_NAME: &str = ".removal-plan.json";
const REMOVAL_PLAN_MAX_BYTES: u64 = 32 * 1024 * 1024;
const REMOVAL_PLAN_FORMAT: &str = "deve.remote-import-removal-plan";
const REMOVAL_PLAN_VERSION: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::remote_import) struct RemoteImportArtifactRemovalPlan {
    repo_id: RepoId,
    logical_epoch: uuid::Uuid,
    root: HostPathIdentity,
    tree_digest: String,
    inventory: HostPathIdentity,
    inventory_digest: String,
    root_quarantine: HostQuarantinePlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::remote_import) struct RemoteImportArtifactRemovalCheckpoint {
    state: RemoteImportArtifactRemovalCheckpointState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum RemoteImportArtifactRemovalCheckpointState {
    Prepared,
    RootQuarantined { root: HostQuarantineCut },
    RootDeleted { root: HostQuarantineCut },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SealedRemovalInventory {
    format: String,
    version: u8,
    repo_id: RepoId,
    logical_epoch: uuid::Uuid,
    invalidated_from_epoch: Option<uuid::Uuid>,
    root: HostPathIdentity,
    tree_digest: String,
    tree: Vec<TreeFingerprint>,
}

impl RemoteImportArtifactRoot {
    pub(in crate::remote_import) fn remove_session_after_inventory(
        &self,
        session_id: RemoteImportSessionId,
    ) -> RemoteImportResult<()> {
        self.verify()?;
        let mut targets = Vec::new();
        let final_path = self.session_path(session_id);
        if path_entry_exists(&final_path)? {
            targets.push(final_path);
        }
        let staging_prefix = format!(".{session_id}.preparing-");
        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&staging_prefix) {
                targets.push(entry.path());
            }
        }
        targets.sort();
        let mut inventories = Vec::new();
        for target in &targets {
            require_direct_child(&self.path, target)?;
            let inventory = inventory_tree(target, None)?;
            let identity = HostPathIdentity::capture(target, HostPathKind::Directory)?;
            let quarantine = HostQuarantinePlan::same_parent(
                identity,
                self.path.join(format!(
                    ".deve-removing-session-{}",
                    uuid::Uuid::new_v4().simple()
                )),
            )?;
            inventories.push((target.clone(), inventory, quarantine));
        }
        self.verify()?;
        for (target, first, _) in &inventories {
            if &inventory_tree(target, None)? != first {
                return Err(RemoteImportError::UnsafeArtifactRoot(format!(
                    "session artifact changed between inventory and cleanup: {target:?}"
                )));
            }
        }
        self.verify()?;
        for (target, _, quarantine) in inventories {
            quarantine.cut()?.delete()?;
            self.verify()?;
            if path_entry_exists(&target)? {
                return Err(RemoteImportError::UnsafeArtifactRoot(format!(
                    "session artifact still exists after cleanup: {target:?}"
                )));
            }
        }
        Ok(())
    }

    pub(in crate::remote_import) fn seal_repo_removal(
        &self,
        repo_id: RepoId,
    ) -> RemoteImportResult<RemoteImportArtifactRemovalPlan> {
        self.verify()?;
        let inventory_path = self.path.join(REMOVAL_PLAN_NAME);
        validate_existing_plan_slot(&inventory_path, repo_id)?;
        let root = HostPathIdentity::capture(&self.path, HostPathKind::Directory)?;
        let tree = inventory_tree(&self.path, Some(&inventory_path))?;
        let tree_digest = inventory_digest(&tree)?;
        let logical_epoch = uuid::Uuid::new_v4();
        let sealed = SealedRemovalInventory {
            format: REMOVAL_PLAN_FORMAT.to_string(),
            version: REMOVAL_PLAN_VERSION,
            repo_id,
            logical_epoch,
            invalidated_from_epoch: None,
            root: root.clone(),
            tree_digest: tree_digest.clone(),
            tree,
        };
        let bytes = encode_inventory(&sealed)?;
        publish_plan_slot(&self.path, &inventory_path, &bytes)?;
        let inventory = HostPathIdentity::capture(&inventory_path, HostPathKind::RegularFile)?;
        if inventory_tree(&self.path, Some(&inventory_path))? != sealed.tree {
            return Err(RemoteImportError::RepoRemovalChanged(repo_id));
        }
        self.verify()?;
        let parent = self.path.parent().ok_or_else(|| {
            RemoteImportError::UnsafeArtifactRoot(
                "repo artifact root has no quarantine parent".to_string(),
            )
        })?;
        let root_quarantine = HostQuarantinePlan::same_parent(
            root.clone(),
            parent.join(format!(
                ".deve-removing-{}-{repo_id}",
                uuid::Uuid::new_v4().simple()
            )),
        )?;
        Ok(RemoteImportArtifactRemovalPlan {
            repo_id,
            logical_epoch,
            root,
            tree_digest,
            inventory,
            inventory_digest: format!("{:x}", Sha256::digest(&bytes)),
            root_quarantine,
        })
    }

    pub(in crate::remote_import) fn revalidate_repo_removal(
        plan: &RemoteImportArtifactRemovalPlan,
    ) -> RemoteImportResult<bool> {
        if !plan.root_quarantine.revalidate_prepared()? {
            return Ok(false);
        }
        if plan.inventory.classify()? != HostPathState::Exact {
            return Ok(false);
        }
        let inventory = read_sealed_inventory(plan)?;
        Ok(inventory_tree(plan.root.path(), Some(plan.inventory.path()))? == inventory.tree)
    }

    /// Pre-cut invalidation changes only the lifecycle-owned logical binding.
    /// The owner slot remains in place and a later stable seal atomically
    /// replaces it, keeping durable debris bounded to one sidecar per RepoId.
    pub(in crate::remote_import) fn invalidate_repo_removal(
        plan: &RemoteImportArtifactRemovalPlan,
    ) -> RemoteImportResult<()> {
        if plan.root.classify()? != HostPathState::Exact {
            return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
        }
        let (identity, bytes, mut inventory) = read_current_inventory_slot(plan)?;
        validate_inventory_stable_binding(plan, &inventory)?;
        if inventory.logical_epoch != plan.logical_epoch {
            return if inventory.invalidated_from_epoch == Some(plan.logical_epoch) {
                crate::utils::fs::sync_directory(plan.root.path())?;
                Ok(())
            } else {
                Err(RemoteImportError::RepoRemovalChanged(plan.repo_id))
            };
        }
        if inventory.invalidated_from_epoch.is_some()
            || identity.object_identity() != plan.inventory.object_identity()
            || format!("{:x}", Sha256::digest(&bytes)) != plan.inventory_digest
        {
            return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
        }
        inventory.invalidated_from_epoch = Some(plan.logical_epoch);
        inventory.logical_epoch = uuid::Uuid::new_v4();
        let bytes = encode_inventory(&inventory)?;
        publish_plan_slot(plan.root.path(), plan.inventory.path(), &bytes)?;
        let (_, _, persisted) = read_current_inventory_slot(plan)?;
        if persisted.invalidated_from_epoch != Some(plan.logical_epoch)
            || persisted.logical_epoch != inventory.logical_epoch
            || validate_inventory_stable_binding(plan, &persisted).is_err()
        {
            return Err(RemoteImportError::UnsafeArtifactRoot(
                "repo artifact removal plan invalidation did not rotate its owner epoch"
                    .to_string(),
            ));
        }
        Ok(())
    }

    pub(in crate::remote_import) fn initial_repo_removal_checkpoint()
    -> RemoteImportArtifactRemovalCheckpoint {
        RemoteImportArtifactRemovalCheckpoint {
            state: RemoteImportArtifactRemovalCheckpointState::Prepared,
        }
    }

    pub(in crate::remote_import) fn advance_repo_removal(
        plan: &RemoteImportArtifactRemovalPlan,
        checkpoint: &RemoteImportArtifactRemovalCheckpoint,
    ) -> RemoteImportResult<RemoteImportArtifactRemovalCheckpoint> {
        let state = match &checkpoint.state {
            RemoteImportArtifactRemovalCheckpointState::Prepared => {
                if let Some(root) = plan.root_quarantine.observe_cut()? {
                    RemoteImportArtifactRemovalCheckpointState::RootQuarantined { root }
                } else if !Self::revalidate_repo_removal(plan)? {
                    return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
                } else {
                    RemoteImportArtifactRemovalCheckpointState::RootQuarantined {
                        root: plan.root_quarantine.cut()?,
                    }
                }
            }
            RemoteImportArtifactRemovalCheckpointState::RootQuarantined { root } => {
                settlement::delete_quarantined_root(plan, root)?;
                RemoteImportArtifactRemovalCheckpointState::RootDeleted { root: root.clone() }
            }
            RemoteImportArtifactRemovalCheckpointState::RootDeleted { root } => {
                if !root.is_deleted()? {
                    return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
                }
                RemoteImportArtifactRemovalCheckpointState::RootDeleted { root: root.clone() }
            }
        };
        Ok(RemoteImportArtifactRemovalCheckpoint { state })
    }

    pub(in crate::remote_import) fn verify_repo_removal_complete(
        plan: &RemoteImportArtifactRemovalPlan,
        checkpoint: &RemoteImportArtifactRemovalCheckpoint,
    ) -> RemoteImportResult<()> {
        let RemoteImportArtifactRemovalCheckpointState::RootDeleted { root } = &checkpoint.state
        else {
            return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
        };
        if !root.belongs_to(&plan.root_quarantine) || !root.is_deleted()? {
            return Err(RemoteImportError::RepoRemovalChanged(plan.repo_id));
        }
        Ok(())
    }
}

impl RemoteImportArtifactRemovalCheckpoint {
    pub(in crate::remote_import) fn is_complete(&self) -> bool {
        matches!(
            self.state,
            RemoteImportArtifactRemovalCheckpointState::RootDeleted { .. }
        )
    }
}

fn publish_plan_slot(root: &Path, destination: &Path, bytes: &[u8]) -> RemoteImportResult<()> {
    let temp = root.join(format!(
        ".removal-plan-{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let mut file = crate::utils::fs::create_atomic_replace_temp(&temp)?;
    let result = (|| -> RemoteImportResult<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        crate::utils::fs::replace_file_atomically(&file, &temp, destination)?;
        crate::utils::fs::sync_directory(root)?;
        Ok(())
    })();
    if result.is_err() {
        drop(file);
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn encode_inventory(inventory: &SealedRemovalInventory) -> RemoteImportResult<Vec<u8>> {
    let bytes = serde_json::to_vec(inventory)
        .map_err(|error| RemoteImportError::UnsafeArtifactRoot(error.to_string()))?;
    if bytes.len() as u64 > REMOVAL_PLAN_MAX_BYTES {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "repo artifact removal inventory exceeds its durable budget".to_string(),
        ));
    }
    Ok(bytes)
}

fn validate_existing_plan_slot(path: &Path, repo_id: RepoId) -> RemoteImportResult<()> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
        Ok(metadata) => {
            if !metadata.is_file()
                || is_reparse(&metadata)
                || metadata.len() > REMOVAL_PLAN_MAX_BYTES
            {
                return Err(RemoteImportError::UnsafeArtifactRoot(
                    "existing removal plan slot is unsafe".to_string(),
                ));
            }
            let mut bytes = Vec::with_capacity(metadata.len() as usize);
            crate::utils::fs::open_regular_file_read(path, "Remote Import removal plan slot")?
                .take(REMOVAL_PLAN_MAX_BYTES + 1)
                .read_to_end(&mut bytes)?;
            let previous: SealedRemovalInventory = serde_json::from_slice(&bytes)
                .map_err(|error| RemoteImportError::UnsafeArtifactRoot(error.to_string()))?;
            if previous.format != REMOVAL_PLAN_FORMAT
                || previous.version != REMOVAL_PLAN_VERSION
                || previous.repo_id != repo_id
            {
                return Err(RemoteImportError::UnsafeArtifactRoot(
                    "existing removal plan slot belongs to an unsupported owner epoch".to_string(),
                ));
            }
            Ok(())
        }
    }
}

fn read_sealed_inventory(
    plan: &RemoteImportArtifactRemovalPlan,
) -> RemoteImportResult<SealedRemovalInventory> {
    validate_inventory_path(plan)?;
    if plan.inventory.classify()? != HostPathState::Exact {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "repo artifact removal inventory identity changed".to_string(),
        ));
    }
    let metadata = std::fs::symlink_metadata(plan.inventory.path())?;
    if metadata.len() > REMOVAL_PLAN_MAX_BYTES {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "repo artifact removal inventory exceeds its read budget".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    crate::utils::fs::open_regular_file_read(
        plan.inventory.path(),
        "Remote Import removal plan slot",
    )?
    .take(REMOVAL_PLAN_MAX_BYTES + 1)
    .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > REMOVAL_PLAN_MAX_BYTES
        || format!("{:x}", Sha256::digest(&bytes)) != plan.inventory_digest
        || plan.inventory.classify()? != HostPathState::Exact
    {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "repo artifact removal inventory content changed".to_string(),
        ));
    }
    let inventory: SealedRemovalInventory = serde_json::from_slice(&bytes)
        .map_err(|error| RemoteImportError::UnsafeArtifactRoot(error.to_string()))?;
    validate_inventory_binding(plan, &inventory)?;
    Ok(inventory)
}

fn read_current_inventory_slot(
    plan: &RemoteImportArtifactRemovalPlan,
) -> RemoteImportResult<(HostPathIdentity, Vec<u8>, SealedRemovalInventory)> {
    validate_inventory_path(plan)?;
    let identity = HostPathIdentity::capture(plan.inventory.path(), HostPathKind::RegularFile)?;
    let metadata = std::fs::symlink_metadata(plan.inventory.path())?;
    if metadata.len() > REMOVAL_PLAN_MAX_BYTES {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "repo artifact removal inventory exceeds its read budget".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    crate::utils::fs::open_regular_file_read(
        plan.inventory.path(),
        "Remote Import removal plan slot",
    )?
    .take(REMOVAL_PLAN_MAX_BYTES + 1)
    .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > REMOVAL_PLAN_MAX_BYTES {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "repo artifact removal inventory exceeds its read budget".to_string(),
        ));
    }
    let inventory = serde_json::from_slice(&bytes)
        .map_err(|error| RemoteImportError::UnsafeArtifactRoot(error.to_string()))?;
    if HostPathIdentity::capture(plan.inventory.path(), HostPathKind::RegularFile)?
        .object_identity()
        != identity.object_identity()
    {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "repo artifact removal inventory identity changed while reading".to_string(),
        ));
    }
    Ok((identity, bytes, inventory))
}

fn validate_inventory_binding(
    plan: &RemoteImportArtifactRemovalPlan,
    inventory: &SealedRemovalInventory,
) -> RemoteImportResult<()> {
    validate_inventory_stable_binding(plan, inventory)?;
    if inventory.logical_epoch != plan.logical_epoch || inventory.invalidated_from_epoch.is_some() {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "repo artifact removal inventory does not bind its active epoch".to_string(),
        ));
    }
    Ok(())
}

fn validate_inventory_stable_binding(
    plan: &RemoteImportArtifactRemovalPlan,
    inventory: &SealedRemovalInventory,
) -> RemoteImportResult<()> {
    if inventory.format != REMOVAL_PLAN_FORMAT
        || inventory.version != REMOVAL_PLAN_VERSION
        || inventory.repo_id != plan.repo_id
        || inventory.root != plan.root
        || inventory.tree_digest != plan.tree_digest
        || inventory_digest(&inventory.tree)? != plan.tree_digest
    {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "repo artifact removal inventory does not bind its opaque plan".to_string(),
        ));
    }
    Ok(())
}

fn validate_inventory_path(plan: &RemoteImportArtifactRemovalPlan) -> RemoteImportResult<()> {
    if plan.inventory.path().parent() != Some(plan.root.path())
        || plan
            .inventory
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            != Some(REMOVAL_PLAN_NAME)
    {
        return Err(RemoteImportError::UnsafeArtifactRoot(
            "repo artifact removal inventory escaped its owner root".to_string(),
        ));
    }
    Ok(())
}

fn path_entry_exists(path: &Path) -> RemoteImportResult<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[cfg(windows)]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x0400 != 0
}

#[cfg(not(windows))]
fn is_reparse(_metadata: &std::fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests;
