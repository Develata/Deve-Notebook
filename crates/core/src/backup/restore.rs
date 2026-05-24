//! plan_ref:
//!   - 18_backup#backup-restore-candidate-contract
//!
//! Restore candidate admission.
//!
//! This module models the point after remote artifacts have been verified,
//! downloaded, and decrypted. It only validates admission metadata; it does
//! not append local ledger entries, stage source-control changes, import
//! branches, merge branches, or touch Projection Workspaces.

use super::locator::{normalize_remote_path, safe_writer_identity};
use super::pack::BackupDigest;
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreAdmissionMode {
    RemoteReadonly,
    ExplicitImport,
    ExplicitMerge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreEvidence {
    pub manifest_verified: bool,
    pub packs_downloaded: bool,
    pub packs_decrypted: bool,
}

impl RestoreEvidence {
    pub fn verified_downloaded_decrypted() -> Self {
        Self {
            manifest_verified: true,
            packs_downloaded: true,
            packs_decrypted: true,
        }
    }

    fn is_complete(self) -> bool {
        self.manifest_verified && self.packs_downloaded && self.packs_decrypted
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreAdmissionState {
    RemoteReadonly,
    ExplicitImport,
    ExplicitMerge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreCandidateInput {
    pub repo_id: RepoId,
    pub expected_repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub manifest_digest: BackupDigest,
    pub pack_count: u64,
    pub pack_digests: Vec<BackupDigest>,
    pub evidence: RestoreEvidence,
    pub admission_mode: RestoreAdmissionMode,
    pub write_gate_confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreCandidate {
    pub repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub manifest_digest: BackupDigest,
    pub pack_count: u64,
    pub pack_digests: Vec<BackupDigest>,
    pub state: RestoreAdmissionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupRestoreError {
    #[error("backup restore candidate repo id does not match expected repo")]
    RepoIdMismatch,
    #[error(
        "backup restore candidate requires verified manifest, downloaded packs, and decrypted packs"
    )]
    IncompleteRestoreEvidence,
    #[error("backup restore candidate must contain at least one pack digest")]
    EmptyRestoreCandidate,
    #[error("backup restore candidate pack count does not match supplied pack digests")]
    PackDigestCountMismatch,
    #[error("backup restore candidate digest must be sha256 hex")]
    InvalidDigest,
    #[error("backup restore candidate pack digest is duplicated")]
    DuplicatePackDigest,
    #[error("backup restore import or merge requires an explicit write gate")]
    WriteGateRequired,
    #[error(transparent)]
    Locator(#[from] super::BackupLocatorError),
}

pub fn admit_restore_candidate(
    input: RestoreCandidateInput,
) -> Result<RestoreCandidate, BackupRestoreError> {
    if input.repo_id != input.expected_repo_id {
        return Err(BackupRestoreError::RepoIdMismatch);
    }
    if !input.evidence.is_complete() {
        return Err(BackupRestoreError::IncompleteRestoreEvidence);
    }
    validate_pack_count(input.pack_count, input.pack_digests.len())?;
    validate_digest(&input.manifest_digest)?;
    validate_pack_digests(&input.pack_digests)?;
    if requires_write_gate(input.admission_mode) && !input.write_gate_confirmed {
        return Err(BackupRestoreError::WriteGateRequired);
    }

    Ok(RestoreCandidate {
        repo_id: input.repo_id,
        writer_identity: safe_writer_identity(&input.writer_identity)?,
        branch_path: normalize_remote_path(&input.branch_path)?,
        manifest_digest: input.manifest_digest,
        pack_count: input.pack_count,
        pack_digests: input.pack_digests,
        state: admission_state(input.admission_mode),
    })
}

fn validate_pack_count(pack_count: u64, actual_count: usize) -> Result<(), BackupRestoreError> {
    if pack_count == 0 || actual_count == 0 {
        return Err(BackupRestoreError::EmptyRestoreCandidate);
    }
    let actual_count = u64::try_from(actual_count).unwrap_or(u64::MAX);
    if pack_count != actual_count {
        return Err(BackupRestoreError::PackDigestCountMismatch);
    }
    Ok(())
}

fn validate_digest(digest: &BackupDigest) -> Result<(), BackupRestoreError> {
    if digest.is_valid_sha256() {
        Ok(())
    } else {
        Err(BackupRestoreError::InvalidDigest)
    }
}

fn validate_pack_digests(pack_digests: &[BackupDigest]) -> Result<(), BackupRestoreError> {
    let mut seen = HashSet::with_capacity(pack_digests.len());
    for digest in pack_digests {
        validate_digest(digest)?;
        let canonical = digest
            .canonical_sha256_hex()
            .ok_or(BackupRestoreError::InvalidDigest)?;
        if !seen.insert(canonical) {
            return Err(BackupRestoreError::DuplicatePackDigest);
        }
    }
    Ok(())
}

fn requires_write_gate(mode: RestoreAdmissionMode) -> bool {
    matches!(
        mode,
        RestoreAdmissionMode::ExplicitImport | RestoreAdmissionMode::ExplicitMerge
    )
}

fn admission_state(mode: RestoreAdmissionMode) -> RestoreAdmissionState {
    match mode {
        RestoreAdmissionMode::RemoteReadonly => RestoreAdmissionState::RemoteReadonly,
        RestoreAdmissionMode::ExplicitImport => RestoreAdmissionState::ExplicitImport,
        RestoreAdmissionMode::ExplicitMerge => RestoreAdmissionState::ExplicitMerge,
    }
}
