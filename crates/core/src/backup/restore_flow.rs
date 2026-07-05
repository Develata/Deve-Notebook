//! plan_ref:
//!   - 06_backup#backup-restore-state-machine-contract
//!
//! Backup restore/import state admission.
//!
//! This module validates restore flow metadata before a restore candidate is
//! admitted. It does not download, decrypt, append ledger entries, stage
//! source-control changes, import branches, merge branches, or touch Projection
//! Workspaces.

use super::locator::{normalize_remote_path, safe_writer_identity};
use super::pack::BackupDigest;
use super::restore::RestoreAdmissionMode;
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupRestoreFlowState {
    RemoteDiscovered,
    ManifestVerified,
    PacksDownloaded,
    PacksDecrypted,
    PacksPlaintextVerified,
    RestoreCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRestoreFlowEvidence {
    pub remote_discovered: bool,
    pub manifest_verified: bool,
    pub packs_downloaded: bool,
    pub packs_decrypted: bool,
    pub packs_plaintext_verified: bool,
    pub candidate_admitted: bool,
}

impl BackupRestoreFlowEvidence {
    pub fn remote_discovered() -> Self {
        Self {
            remote_discovered: true,
            manifest_verified: false,
            packs_downloaded: false,
            packs_decrypted: false,
            packs_plaintext_verified: false,
            candidate_admitted: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRestoreFlowInput {
    pub expected_repo_id: RepoId,
    pub manifest_repo_id: Option<RepoId>,
    pub writer_identity: String,
    pub branch_path: String,
    pub manifest_digest: Option<BackupDigest>,
    pub pack_digests: Vec<BackupDigest>,
    pub evidence: BackupRestoreFlowEvidence,
    pub admission_mode: RestoreAdmissionMode,
    pub write_gate_confirmed: bool,
    pub local_ledger_append_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRestoreFlowPlan {
    pub repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub manifest_digest: Option<BackupDigest>,
    pub pack_count: u64,
    pub state: BackupRestoreFlowState,
    pub admission_mode: RestoreAdmissionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupRestoreFlowError {
    #[error("backup restore remote has not been discovered")]
    RemoteNotDiscovered,
    #[error("backup restore evidence is out of state-machine order")]
    EvidenceOutOfOrder,
    #[error("backup restore manifest metadata is required after manifest verification")]
    MissingManifestMetadata,
    #[error("backup restore manifest repo id does not match expected repo")]
    RepoIdMismatch,
    #[error("backup restore downloaded pack list is empty")]
    EmptyPackDownload,
    #[error("backup restore digest must be sha256 hex")]
    InvalidDigest,
    #[error("backup restore pack digest is duplicated")]
    DuplicatePackDigest,
    #[error("backup restore download/decrypt phase must not append local ledger state")]
    LocalLedgerAppendForbidden,
    #[error("backup restore import or merge requires an explicit write gate")]
    WriteGateRequired,
    #[error(transparent)]
    Locator(#[from] super::BackupLocatorError),
}

pub fn plan_backup_restore_flow(
    input: BackupRestoreFlowInput,
) -> Result<BackupRestoreFlowPlan, BackupRestoreFlowError> {
    validate_evidence_order(input.evidence)?;
    validate_manifest_metadata(
        input.evidence,
        input.expected_repo_id,
        input.manifest_repo_id,
        input.manifest_digest.as_ref(),
    )?;
    validate_pack_digests(input.evidence, &input.pack_digests)?;
    validate_local_append_gate(
        input.evidence,
        input.admission_mode,
        input.write_gate_confirmed,
        input.local_ledger_append_requested,
    )?;

    Ok(BackupRestoreFlowPlan {
        repo_id: input.expected_repo_id,
        writer_identity: safe_writer_identity(&input.writer_identity)?,
        branch_path: normalize_remote_path(&input.branch_path)?,
        manifest_digest: input.manifest_digest,
        pack_count: u64::try_from(input.pack_digests.len()).unwrap_or(u64::MAX),
        state: flow_state(input.evidence),
        admission_mode: input.admission_mode,
    })
}

fn validate_evidence_order(
    evidence: BackupRestoreFlowEvidence,
) -> Result<(), BackupRestoreFlowError> {
    if !evidence.remote_discovered {
        return Err(BackupRestoreFlowError::RemoteNotDiscovered);
    }
    if (evidence.packs_downloaded && !evidence.manifest_verified)
        || (evidence.packs_decrypted && !evidence.packs_downloaded)
        || (evidence.packs_plaintext_verified && !evidence.packs_decrypted)
        || (evidence.candidate_admitted && !evidence.packs_plaintext_verified)
    {
        return Err(BackupRestoreFlowError::EvidenceOutOfOrder);
    }
    Ok(())
}

fn validate_manifest_metadata(
    evidence: BackupRestoreFlowEvidence,
    expected_repo_id: RepoId,
    manifest_repo_id: Option<RepoId>,
    manifest_digest: Option<&BackupDigest>,
) -> Result<(), BackupRestoreFlowError> {
    if !evidence.manifest_verified {
        return Ok(());
    }
    let Some(manifest_repo_id) = manifest_repo_id else {
        return Err(BackupRestoreFlowError::MissingManifestMetadata);
    };
    let Some(manifest_digest) = manifest_digest else {
        return Err(BackupRestoreFlowError::MissingManifestMetadata);
    };
    if manifest_repo_id != expected_repo_id {
        return Err(BackupRestoreFlowError::RepoIdMismatch);
    }
    validate_digest(manifest_digest)
}

fn validate_pack_digests(
    evidence: BackupRestoreFlowEvidence,
    pack_digests: &[BackupDigest],
) -> Result<(), BackupRestoreFlowError> {
    let mut seen = HashSet::with_capacity(pack_digests.len());
    for digest in pack_digests {
        validate_digest(digest)?;
        let canonical = digest
            .canonical_sha256_hex()
            .ok_or(BackupRestoreFlowError::InvalidDigest)?;
        if !seen.insert(canonical) {
            return Err(BackupRestoreFlowError::DuplicatePackDigest);
        }
    }
    if !evidence.packs_downloaded
        && !evidence.packs_decrypted
        && !evidence.packs_plaintext_verified
        && !evidence.candidate_admitted
    {
        return Ok(());
    }
    if pack_digests.is_empty() {
        return Err(BackupRestoreFlowError::EmptyPackDownload);
    }
    Ok(())
}

fn validate_local_append_gate(
    evidence: BackupRestoreFlowEvidence,
    admission_mode: RestoreAdmissionMode,
    write_gate_confirmed: bool,
    local_ledger_append_requested: bool,
) -> Result<(), BackupRestoreFlowError> {
    if matches!(
        admission_mode,
        RestoreAdmissionMode::ExplicitImport | RestoreAdmissionMode::ExplicitMerge
    ) && evidence.candidate_admitted
        && !write_gate_confirmed
    {
        return Err(BackupRestoreFlowError::WriteGateRequired);
    }

    if !local_ledger_append_requested {
        return Ok(());
    }
    if !evidence.candidate_admitted
        || admission_mode == RestoreAdmissionMode::RemoteReadonly
        || !write_gate_confirmed
    {
        return Err(BackupRestoreFlowError::LocalLedgerAppendForbidden);
    }
    Ok(())
}

fn validate_digest(digest: &BackupDigest) -> Result<(), BackupRestoreFlowError> {
    if digest.is_valid_sha256() {
        Ok(())
    } else {
        Err(BackupRestoreFlowError::InvalidDigest)
    }
}

fn flow_state(evidence: BackupRestoreFlowEvidence) -> BackupRestoreFlowState {
    if evidence.candidate_admitted {
        return BackupRestoreFlowState::RestoreCandidate;
    }
    if evidence.packs_plaintext_verified {
        return BackupRestoreFlowState::PacksPlaintextVerified;
    }
    if evidence.packs_decrypted {
        return BackupRestoreFlowState::PacksDecrypted;
    }
    if evidence.packs_downloaded {
        return BackupRestoreFlowState::PacksDownloaded;
    }
    if evidence.manifest_verified {
        return BackupRestoreFlowState::ManifestVerified;
    }
    BackupRestoreFlowState::RemoteDiscovered
}
