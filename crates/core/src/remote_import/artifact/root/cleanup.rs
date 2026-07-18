//! plan_ref:
//!   - 03_storage/repair#remote-import-cleanup-repair

use super::{
    MAX_ARTIFACT_TREE_DEPTH, MAX_ARTIFACT_TREE_NODES, RemoteImportArtifactRoot, is_reparse,
    require_direct_child, validate_existing_directory,
};
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::types::RemoteImportSessionId;
use std::path::{Path, PathBuf};

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
            inventories.push((target.clone(), inventory_tree(target)?));
        }
        self.verify()?;
        for (target, first) in &inventories {
            let second = inventory_tree(target)?;
            if first != &second {
                return Err(RemoteImportError::UnsafeArtifactRoot(format!(
                    "session artifact changed between inventory and cleanup: {:?}",
                    target
                )));
            }
        }
        self.verify()?;
        for (target, _) in inventories {
            std::fs::remove_dir_all(&target)?;
            self.verify()?;
            if path_entry_exists(&target)? {
                return Err(RemoteImportError::UnsafeArtifactRoot(format!(
                    "session artifact still exists after cleanup: {:?}",
                    target
                )));
            }
        }
        Ok(())
    }
}

fn path_entry_exists(path: &Path) -> RemoteImportResult<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeFingerprint {
    relative: PathBuf,
    kind: u8,
    len: u64,
    identity: file_id::FileId,
}

fn inventory_tree(root: &Path) -> RemoteImportResult<Vec<TreeFingerprint>> {
    validate_existing_directory(root)?;
    let mut fingerprints = Vec::new();
    inventory_tree_inner(root, root, 0, &mut fingerprints)?;
    fingerprints.sort_by(|left, right| left.relative.cmp(&right.relative));
    Ok(fingerprints)
}

fn inventory_tree_inner(
    root: &Path,
    path: &Path,
    depth: usize,
    fingerprints: &mut Vec<TreeFingerprint>,
) -> RemoteImportResult<()> {
    if fingerprints.len() >= MAX_ARTIFACT_TREE_NODES || depth > MAX_ARTIFACT_TREE_DEPTH {
        return Err(RemoteImportError::UnsafeArtifactRoot(format!(
            "cleanup inventory exceeds safety bound at {:?}",
            path
        )));
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(RemoteImportError::UnsafeArtifactRoot(format!(
            "cleanup inventory found symlink/reparse object: {:?}",
            path
        )));
    }
    let relative = path
        .strip_prefix(root)
        .map_err(|error| RemoteImportError::UnsafeArtifactRoot(error.to_string()))?
        .to_path_buf();
    let kind = if metadata.is_dir() {
        1
    } else if metadata.is_file() {
        2
    } else {
        return Err(RemoteImportError::UnsafeArtifactRoot(format!(
            "cleanup inventory found unsupported object: {:?}",
            path
        )));
    };
    let identity = file_id::get_file_id(path).map_err(|error| {
        RemoteImportError::UnsafeArtifactRoot(format!(
            "failed to fingerprint artifact {:?}: {}",
            path, error
        ))
    })?;
    fingerprints.push(TreeFingerprint {
        relative,
        kind,
        len: metadata.len(),
        identity,
    });
    if metadata.is_dir() {
        let mut children = std::fs::read_dir(path)?
            .map(|entry| entry.map(|value| value.path()))
            .collect::<Result<Vec<_>, _>>()?;
        children.sort();
        for child in children {
            inventory_tree_inner(root, &child, depth + 1, fingerprints)?;
        }
    }
    Ok(())
}
