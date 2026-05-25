//! plan_ref:
//!   - 06_backup#backup-pack-contract
//!
//! Backup pack manifest planning and validation.
//!
//! This module is deliberately pure data validation. It does not encrypt,
//! upload, download, append ledger entries, write source-control state, or
//! touch Projection Workspaces.

use super::locator::{normalize_remote_path, safe_writer_identity};
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod blob_ref;
#[cfg(test)]
mod tests;

use blob_ref::{normalize_blob_refs, validate_blob_refs};

pub const BACKUP_PACK_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupDigest {
    pub algorithm: String,
    pub hex: String,
}

impl BackupDigest {
    pub fn sha256(hex: impl Into<String>) -> Self {
        Self {
            algorithm: "sha256".into(),
            hex: hex.into(),
        }
    }

    pub fn is_valid_sha256(&self) -> bool {
        self.algorithm == "sha256"
            && self.hex.len() == 64
            && self.hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    }

    pub fn same_sha256(&self, other: &Self) -> bool {
        self.is_valid_sha256()
            && other.is_valid_sha256()
            && self.hex.eq_ignore_ascii_case(&other.hex)
    }

    pub fn canonical_sha256_hex(&self) -> Option<String> {
        if self.is_valid_sha256() {
            Some(self.hex.to_ascii_lowercase())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSeqRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBlobRef {
    pub path: String,
    pub size_bytes: u64,
    pub digest: BackupDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupPackPlanInput {
    pub repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub pack_sequence: u64,
    pub ledger_seq_range: Option<BackupSeqRange>,
    pub ledger_event_count: u64,
    pub snapshot_count: u64,
    pub payload_digest: BackupDigest,
    pub blob_refs: Vec<BackupBlobRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupPackManifest {
    pub format_version: u32,
    pub repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub pack_sequence: u64,
    pub pack_file_name: String,
    pub ledger_seq_range: Option<BackupSeqRange>,
    pub ledger_event_count: u64,
    pub snapshot_count: u64,
    pub payload_digest: BackupDigest,
    pub blob_refs: Vec<BackupBlobRef>,
}

impl BackupPackManifest {
    pub fn pack_object_path(&self) -> String {
        format!("{}/packs/{}", self.branch_path, self.pack_file_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupPackError {
    #[error("backup pack format version is unsupported: {0}")]
    UnsupportedFormatVersion(u32),
    #[error("backup pack repo id does not match expected repo")]
    RepoIdMismatch,
    #[error("backup pack writer identity does not match expected writer")]
    WriterIdentityMismatch,
    #[error("backup pack branch path does not match expected branch")]
    BranchPathMismatch,
    #[error("backup pack sequence must be greater than zero")]
    InvalidPackSequence,
    #[error("backup pack must contain ledger events, snapshots, or blob refs")]
    EmptyPack,
    #[error("backup pack ledger sequence range is invalid")]
    InvalidLedgerRange,
    #[error("backup pack digest must be sha256 hex")]
    InvalidDigest,
    #[error("backup pack blob path is duplicated")]
    DuplicateBlobPath,
    #[error(transparent)]
    Locator(#[from] super::BackupLocatorError),
}

pub fn plan_backup_pack(input: BackupPackPlanInput) -> Result<BackupPackManifest, BackupPackError> {
    validate_pack_sequence(input.pack_sequence)?;
    validate_artifact_counts(
        input.ledger_event_count,
        input.snapshot_count,
        &input.blob_refs,
    )?;
    validate_ledger_range(input.ledger_event_count, input.ledger_seq_range)?;
    validate_digest(&input.payload_digest)?;

    let writer_identity = safe_writer_identity(&input.writer_identity)?;
    let branch_path = normalize_remote_path(&input.branch_path)?;
    let blob_refs = normalize_blob_refs(input.blob_refs)?;

    Ok(BackupPackManifest {
        format_version: BACKUP_PACK_FORMAT_VERSION,
        repo_id: input.repo_id,
        writer_identity,
        branch_path,
        pack_sequence: input.pack_sequence,
        pack_file_name: pack_file_name(input.pack_sequence),
        ledger_seq_range: input.ledger_seq_range,
        ledger_event_count: input.ledger_event_count,
        snapshot_count: input.snapshot_count,
        payload_digest: input.payload_digest,
        blob_refs,
    })
}

pub fn validate_pack_manifest(
    manifest: &BackupPackManifest,
    expected_repo_id: RepoId,
    expected_writer_identity: &str,
    expected_branch_path: &str,
) -> Result<(), BackupPackError> {
    if manifest.format_version != BACKUP_PACK_FORMAT_VERSION {
        return Err(BackupPackError::UnsupportedFormatVersion(
            manifest.format_version,
        ));
    }
    if manifest.repo_id != expected_repo_id {
        return Err(BackupPackError::RepoIdMismatch);
    }
    if manifest.writer_identity != safe_writer_identity(expected_writer_identity)? {
        return Err(BackupPackError::WriterIdentityMismatch);
    }
    if manifest.branch_path != normalize_remote_path(expected_branch_path)? {
        return Err(BackupPackError::BranchPathMismatch);
    }
    validate_pack_sequence(manifest.pack_sequence)?;
    if manifest.pack_file_name != pack_file_name(manifest.pack_sequence) {
        return Err(BackupPackError::InvalidPackSequence);
    }
    validate_artifact_counts(
        manifest.ledger_event_count,
        manifest.snapshot_count,
        &manifest.blob_refs,
    )?;
    validate_ledger_range(manifest.ledger_event_count, manifest.ledger_seq_range)?;
    validate_digest(&manifest.payload_digest)?;
    validate_blob_refs(&manifest.blob_refs)?;
    Ok(())
}

pub(super) fn pack_file_name(pack_sequence: u64) -> String {
    format!("{pack_sequence:06}.pack.enc")
}

fn validate_pack_sequence(pack_sequence: u64) -> Result<(), BackupPackError> {
    if pack_sequence == 0 {
        return Err(BackupPackError::InvalidPackSequence);
    }
    Ok(())
}

fn validate_artifact_counts(
    ledger_event_count: u64,
    snapshot_count: u64,
    blob_refs: &[BackupBlobRef],
) -> Result<(), BackupPackError> {
    if ledger_event_count == 0 && snapshot_count == 0 && blob_refs.is_empty() {
        return Err(BackupPackError::EmptyPack);
    }
    Ok(())
}

fn validate_ledger_range(
    ledger_event_count: u64,
    ledger_seq_range: Option<BackupSeqRange>,
) -> Result<(), BackupPackError> {
    match (ledger_event_count, ledger_seq_range) {
        (0, None) => Ok(()),
        (0, Some(_)) | (_, None) => Err(BackupPackError::InvalidLedgerRange),
        (_, Some(range)) if range.start <= range.end => Ok(()),
        (_, Some(_)) => Err(BackupPackError::InvalidLedgerRange),
    }
}

fn validate_digest(digest: &BackupDigest) -> Result<(), BackupPackError> {
    if !digest.is_valid_sha256() {
        return Err(BackupPackError::InvalidDigest);
    }
    Ok(())
}
