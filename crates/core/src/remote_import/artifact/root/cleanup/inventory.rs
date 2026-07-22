//! plan_ref:
//!   - 03_storage/repair#remote-import-cleanup-repair
//!
//! Bounded no-follow identity inventory for one Remote Import removal sidecar.

use super::super::{
    MAX_ARTIFACT_TREE_DEPTH, MAX_ARTIFACT_TREE_NODES, is_reparse, validate_existing_directory,
};
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::utils::fs::{HostPathIdentity, HostPathKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TreeFingerprint {
    pub(super) relative: PathBuf,
    pub(super) kind: u8,
    pub(super) len: u64,
    pub(super) identity: HostPathIdentity,
}

pub(super) fn inventory_digest(inventory: &[TreeFingerprint]) -> RemoteImportResult<String> {
    let bytes = serde_json::to_vec(inventory)
        .map_err(|error| RemoteImportError::UnsafeArtifactRoot(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(super) fn inventory_tree(
    root: &Path,
    excluded: Option<&Path>,
) -> RemoteImportResult<Vec<TreeFingerprint>> {
    validate_existing_directory(root)?;
    let mut fingerprints = Vec::new();
    inventory_tree_inner(root, root, excluded, 0, &mut fingerprints)?;
    fingerprints.sort_by(|left, right| left.relative.cmp(&right.relative));
    Ok(fingerprints)
}

fn inventory_tree_inner(
    root: &Path,
    path: &Path,
    excluded: Option<&Path>,
    depth: usize,
    fingerprints: &mut Vec<TreeFingerprint>,
) -> RemoteImportResult<()> {
    if excluded == Some(path) {
        return Ok(());
    }
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
    let identity = HostPathIdentity::capture(
        path,
        if metadata.is_dir() {
            HostPathKind::Directory
        } else {
            HostPathKind::RegularFile
        },
    )?;
    fingerprints.push(TreeFingerprint {
        relative,
        kind,
        // Directory lengths are filesystem bookkeeping rather than content
        // size (for example NTFS may change 0 to 4096 after an atomic child
        // replacement). Directory identity and the recursively inventoried
        // child set provide the stable binding; file lengths remain exact.
        len: if metadata.is_file() {
            metadata.len()
        } else {
            0
        },
        identity,
    });
    if metadata.is_dir() {
        let mut children = std::fs::read_dir(path)?
            .map(|entry| entry.map(|value| value.path()))
            .collect::<Result<Vec<_>, _>>()?;
        children.sort();
        for child in children {
            inventory_tree_inner(root, &child, excluded, depth + 1, fingerprints)?;
        }
    }
    Ok(())
}
