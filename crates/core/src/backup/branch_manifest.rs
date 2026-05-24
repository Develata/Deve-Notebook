//! plan_ref:
//!   - 18_backup#backup-branch-binding-contract
//!   - 18_backup#backup-remote-layout-contract
//!   - 18_backup#backup-pack-contract
//!
//! Backup branch manifest validation.
//!
//! This module validates branch manifest metadata and pack object references
//! only. It does not fetch providers, decrypt artifacts, append ledger entries,
//! stage source-control changes, or touch Projection Workspaces.

use super::locator::{BranchBackupLocator, normalize_remote_path, safe_writer_identity};
use super::pack::{BackupDigest, pack_file_name};
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[cfg(test)]
mod tests;

pub const BACKUP_BRANCH_MANIFEST_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBranchManifestPackRef {
    pub pack_sequence: u64,
    pub object_path: String,
    pub payload_digest: BackupDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBranchManifestInput {
    pub branch: BranchBackupLocator,
    pub expected_repo_id: RepoId,
    pub manifest_repo_id: RepoId,
    pub manifest_writer_identity: String,
    pub manifest_branch_path: String,
    pub format_version: u32,
    pub packs: Vec<BackupBranchManifestPackRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBranchManifest {
    pub repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub branch_manifest_path: String,
    pub pack_prefix: String,
    pub format_version: u32,
    pub packs: Vec<BackupBranchManifestPackRef>,
}

impl BackupBranchManifest {
    pub fn expected_pack_object_paths(&self) -> Vec<String> {
        self.packs
            .iter()
            .map(|pack| pack.object_path.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupBranchManifestError {
    #[error("backup branch manifest format version is unsupported: {0}")]
    UnsupportedFormatVersion(u32),
    #[error("backup branch manifest repo id does not match expected repo")]
    RepoIdMismatch,
    #[error("backup branch manifest writer identity does not match locator writer")]
    WriterIdentityMismatch,
    #[error("backup branch manifest path does not match locator branch path")]
    BranchPathMismatch,
    #[error("backup branch manifest must contain at least one pack")]
    EmptyPackList,
    #[error("backup branch manifest pack sequence must be greater than zero")]
    InvalidPackSequence,
    #[error("backup branch manifest pack sequence is duplicated")]
    DuplicatePackSequence,
    #[error("backup branch manifest pack object path is duplicated")]
    DuplicatePackObjectPath,
    #[error("backup branch manifest pack object path is outside branch pack prefix")]
    PackPathOutsideBranchPrefix,
    #[error("backup branch manifest pack object path does not match pack sequence")]
    PackObjectPathMismatch,
    #[error("backup branch manifest digest must be sha256 hex")]
    InvalidDigest,
    #[error(transparent)]
    Locator(#[from] super::BackupLocatorError),
}

pub fn validate_backup_branch_manifest(
    input: BackupBranchManifestInput,
) -> Result<BackupBranchManifest, BackupBranchManifestError> {
    if input.format_version != BACKUP_BRANCH_MANIFEST_FORMAT_VERSION {
        return Err(BackupBranchManifestError::UnsupportedFormatVersion(
            input.format_version,
        ));
    }
    if input.manifest_repo_id != input.expected_repo_id {
        return Err(BackupBranchManifestError::RepoIdMismatch);
    }

    let writer_identity = safe_writer_identity(&input.manifest_writer_identity)?;
    if writer_identity != input.branch.writer_identity {
        return Err(BackupBranchManifestError::WriterIdentityMismatch);
    }

    let branch_path = normalize_remote_path(&input.manifest_branch_path)?;
    if branch_path != input.branch.branch_path {
        return Err(BackupBranchManifestError::BranchPathMismatch);
    }

    let pack_prefix = input.branch.pack_prefix();
    let packs = validate_pack_refs(&pack_prefix, input.packs)?;

    Ok(BackupBranchManifest {
        repo_id: input.expected_repo_id,
        writer_identity,
        branch_path,
        branch_manifest_path: input.branch.branch_manifest_path(),
        pack_prefix,
        format_version: BACKUP_BRANCH_MANIFEST_FORMAT_VERSION,
        packs,
    })
}

fn validate_pack_refs(
    pack_prefix: &str,
    packs: Vec<BackupBranchManifestPackRef>,
) -> Result<Vec<BackupBranchManifestPackRef>, BackupBranchManifestError> {
    if packs.is_empty() {
        return Err(BackupBranchManifestError::EmptyPackList);
    }

    let mut sequences = HashSet::with_capacity(packs.len());
    let mut object_paths = HashSet::with_capacity(packs.len());
    let mut normalized = Vec::with_capacity(packs.len());
    for pack in packs {
        if pack.pack_sequence == 0 {
            return Err(BackupBranchManifestError::InvalidPackSequence);
        }
        if !sequences.insert(pack.pack_sequence) {
            return Err(BackupBranchManifestError::DuplicatePackSequence);
        }
        if !pack.payload_digest.is_valid_sha256() {
            return Err(BackupBranchManifestError::InvalidDigest);
        }

        let object_path = normalize_remote_path(&pack.object_path)?;
        if object_path == pack_prefix || !object_path.starts_with(&format!("{pack_prefix}/")) {
            return Err(BackupBranchManifestError::PackPathOutsideBranchPrefix);
        }
        if !object_paths.insert(object_path.clone()) {
            return Err(BackupBranchManifestError::DuplicatePackObjectPath);
        }
        if object_path != format!("{pack_prefix}/{}", pack_file_name(pack.pack_sequence)) {
            return Err(BackupBranchManifestError::PackObjectPathMismatch);
        }

        normalized.push(BackupBranchManifestPackRef {
            pack_sequence: pack.pack_sequence,
            object_path,
            payload_digest: pack.payload_digest,
        });
    }
    Ok(normalized)
}
