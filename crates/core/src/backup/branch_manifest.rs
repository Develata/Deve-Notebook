//! plan_ref:
//!   - 06_backup#backup-branch-binding-contract
//!   - 06_backup#backup-remote-layout-contract
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-artifact-protection-contract
//!
//! Backup branch manifest validation and encrypted artifact opening.
//!
//! This module owns the branch manifest data model. Validation and encrypted
//! artifact handling live in sibling modules so the manifest format remains
//! separate from transport/decrypt evidence gates.

mod artifact;
mod validation;

#[cfg(test)]
mod tests;

pub use artifact::{
    BackupBranchManifestArtifactError, BackupBranchManifestArtifactInput,
    BackupBranchManifestArtifactOpenInput, BackupBranchManifestArtifactOpenResult,
    BackupEncryptedBranchManifestArtifact, encrypt_backup_branch_manifest_artifact,
    open_backup_branch_manifest_artifact,
};
pub use validation::{BackupBranchManifestError, validate_backup_branch_manifest};

use super::pack::{
    BackupBlobRef, BackupDigest, BackupPackManifest, BackupSeqRange, pack_file_name,
    validate_pack_manifest,
};
use crate::models::RepoId;
use serde::{Deserialize, Serialize};

pub const BACKUP_BRANCH_MANIFEST_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupBranchManifestPackRef {
    pub pack_sequence: u64,
    pub object_path: String,
    pub payload_digest: BackupDigest,
    pub ledger_seq_range: Option<BackupSeqRange>,
    pub ledger_event_count: u64,
    pub snapshot_count: u64,
    pub blob_refs: Vec<BackupBlobRef>,
}

impl BackupBranchManifestPackRef {
    pub fn from_pack_manifest(manifest: &BackupPackManifest) -> Self {
        Self {
            pack_sequence: manifest.pack_sequence,
            object_path: manifest.pack_object_path(),
            payload_digest: manifest.payload_digest.clone(),
            ledger_seq_range: manifest.ledger_seq_range,
            ledger_event_count: manifest.ledger_event_count,
            snapshot_count: manifest.snapshot_count,
            blob_refs: manifest.blob_refs.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBranchManifestInput {
    pub branch: super::BranchBackupLocator,
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

    pub fn pack_manifest_for_ref(
        &self,
        pack_ref: &BackupBranchManifestPackRef,
    ) -> Result<BackupPackManifest, BackupBranchManifestError> {
        let manifest = BackupPackManifest {
            format_version: super::pack::BACKUP_PACK_FORMAT_VERSION,
            repo_id: self.repo_id,
            writer_identity: self.writer_identity.clone(),
            branch_path: self.branch_path.clone(),
            pack_sequence: pack_ref.pack_sequence,
            pack_file_name: pack_file_name(pack_ref.pack_sequence),
            ledger_seq_range: pack_ref.ledger_seq_range,
            ledger_event_count: pack_ref.ledger_event_count,
            snapshot_count: pack_ref.snapshot_count,
            payload_digest: pack_ref.payload_digest.clone(),
            blob_refs: pack_ref.blob_refs.clone(),
        };
        validate_pack_manifest(
            &manifest,
            self.repo_id,
            &self.writer_identity,
            &self.branch_path,
        )?;
        if manifest.pack_object_path() != pack_ref.object_path {
            return Err(BackupBranchManifestError::PackObjectPathMismatch);
        }
        Ok(manifest)
    }
}
