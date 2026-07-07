//! plan_ref:
//!   - 06_backup#backup-restore-candidate-contract
//!
//! Restore candidate admission.
//!
//! This module models the point after remote artifacts have been verified,
//! downloaded, and decrypted. It only validates admission metadata; it does
//! not append local ledger entries, stage source-control changes, import
//! branches, merge branches, or touch Projection Workspaces.

use super::locator::{normalize_remote_path, safe_writer_identity};
use super::pack::BackupDigest;
use super::verification::{BackupPlaintextPacksResult, BackupVerificationResult};
use crate::models::RepoId;
use crate::security::hashing::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[cfg(test)]
pub(crate) mod tests;

pub const BACKUP_RESTORE_MAX_PACKS: u64 = 64;
pub const BACKUP_RESTORE_MAX_ENCRYPTED_BYTES: usize = 128 * 1024 * 1024;
pub const BACKUP_RESTORE_MAX_PLAINTEXT_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreAdmissionMode {
    RemoteReadonly,
    ExplicitImport,
    ExplicitMerge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct RestoreEvidence {
    pub manifest_verified: bool,
    pub packs_downloaded: bool,
    pub packs_decrypted: bool,
    pub packs_plaintext_verified: bool,
}

impl RestoreEvidence {
    fn verified_downloaded_decrypted_plaintext() -> Self {
        Self {
            manifest_verified: true,
            packs_downloaded: true,
            packs_decrypted: true,
            packs_plaintext_verified: true,
        }
    }

    fn is_complete(self) -> bool {
        self.manifest_verified
            && self.packs_downloaded
            && self.packs_decrypted
            && self.packs_plaintext_verified
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreAdmissionState {
    RemoteReadonly,
    ExplicitImport,
    ExplicitMerge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RestoreCandidateInput {
    pub repo_id: RepoId,
    pub expected_repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub manifest_digest: BackupDigest,
    pub pack_count: u64,
    pub pack_digests: Vec<BackupDigest>,
    pub plaintext_evidence_digest: BackupDigest,
    pub evidence: RestoreEvidence,
    pub admission_mode: RestoreAdmissionMode,
    pub write_gate_confirmed: bool,
}

#[derive(Debug)]
pub struct RestoreCandidateFromVerifiedPacksInput<'a> {
    pub expected_repo_id: RepoId,
    pub manifest_verification: &'a BackupVerificationResult,
    pub plaintext_packs: &'a BackupPlaintextPacksResult,
    pub admission_mode: RestoreAdmissionMode,
    pub write_gate_confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupRestoreResourceBudgetInput {
    pub pack_count: u64,
    pub encrypted_bytes: usize,
    pub plaintext_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreCandidate {
    pub repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub manifest_digest: BackupDigest,
    pub pack_count: u64,
    pub pack_digests: Vec<BackupDigest>,
    pub plaintext_evidence_digest: BackupDigest,
    pub fingerprint: BackupDigest,
    pub state: RestoreAdmissionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupRestoreError {
    #[error("backup restore candidate repo id does not match expected repo")]
    RepoIdMismatch,
    #[error(
        "backup restore candidate requires verified manifest, downloaded packs, decrypted packs, and plaintext schema verification"
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
    #[error("backup restore candidate fingerprint digest is invalid")]
    InvalidCandidateFingerprint,
    #[error("backup restore candidate typed verification evidence does not match plaintext packs")]
    TypedEvidenceMismatch,
    #[error("backup restore candidate pack count exceeds resource budget")]
    PackCountBudgetExceeded,
    #[error("backup restore candidate encrypted bytes exceed resource budget")]
    EncryptedBytesBudgetExceeded,
    #[error("backup restore candidate plaintext bytes exceed resource budget")]
    PlaintextBytesBudgetExceeded,
    #[error("backup restore import or merge requires an explicit write gate")]
    WriteGateRequired,
    #[error(transparent)]
    Locator(#[from] super::BackupLocatorError),
}

fn admit_restore_candidate(
    input: RestoreCandidateInput,
) -> Result<RestoreCandidate, BackupRestoreError> {
    if input.repo_id != input.expected_repo_id {
        return Err(BackupRestoreError::RepoIdMismatch);
    }
    if !input.evidence.is_complete() {
        return Err(BackupRestoreError::IncompleteRestoreEvidence);
    }
    validate_pack_count(input.pack_count, input.pack_digests.len())?;
    validate_backup_restore_resource_budget(BackupRestoreResourceBudgetInput {
        pack_count: input.pack_count,
        encrypted_bytes: 0,
        plaintext_bytes: 0,
    })?;
    validate_digest(&input.manifest_digest)?;
    validate_pack_digests(&input.pack_digests)?;
    validate_digest(&input.plaintext_evidence_digest)?;
    if requires_write_gate(input.admission_mode) && !input.write_gate_confirmed {
        return Err(BackupRestoreError::WriteGateRequired);
    }

    let writer_identity = safe_writer_identity(&input.writer_identity)?;
    let branch_path = normalize_remote_path(&input.branch_path)?;
    let fingerprint = restore_candidate_fingerprint(
        input.repo_id,
        &writer_identity,
        &branch_path,
        &input.manifest_digest,
        input.pack_count,
        &input.pack_digests,
        &input.plaintext_evidence_digest,
    )?;

    Ok(RestoreCandidate {
        repo_id: input.repo_id,
        writer_identity,
        branch_path,
        manifest_digest: input.manifest_digest,
        pack_count: input.pack_count,
        pack_digests: input.pack_digests,
        plaintext_evidence_digest: input.plaintext_evidence_digest,
        fingerprint,
        state: admission_state(input.admission_mode),
    })
}

pub fn admit_verified_restore_candidate(
    input: RestoreCandidateFromVerifiedPacksInput<'_>,
) -> Result<RestoreCandidate, BackupRestoreError> {
    validate_typed_evidence(
        input.expected_repo_id,
        input.manifest_verification,
        input.plaintext_packs,
    )?;
    validate_backup_restore_resource_budget(BackupRestoreResourceBudgetInput {
        pack_count: input.plaintext_packs.pack_count(),
        encrypted_bytes: input.plaintext_packs.encrypted_bytes_total(),
        plaintext_bytes: input.plaintext_packs.plaintext_bytes_total(),
    })?;

    admit_restore_candidate(RestoreCandidateInput {
        repo_id: input.plaintext_packs.repo_id(),
        expected_repo_id: input.expected_repo_id,
        writer_identity: input.plaintext_packs.writer_identity().to_owned(),
        branch_path: input.plaintext_packs.branch_path().to_owned(),
        manifest_digest: input.manifest_verification.manifest_digest().clone(),
        pack_count: input.plaintext_packs.pack_count(),
        pack_digests: input.plaintext_packs.pack_digests().to_vec(),
        plaintext_evidence_digest: plaintext_evidence_digest(input.plaintext_packs)?,
        evidence: RestoreEvidence::verified_downloaded_decrypted_plaintext(),
        admission_mode: input.admission_mode,
        write_gate_confirmed: input.write_gate_confirmed,
    })
}

pub(crate) fn verify_restore_candidate_fingerprint(
    candidate: &RestoreCandidate,
) -> Result<(), BackupRestoreError> {
    let expected = restore_candidate_fingerprint(
        candidate.repo_id,
        &candidate.writer_identity,
        &candidate.branch_path,
        &candidate.manifest_digest,
        candidate.pack_count,
        &candidate.pack_digests,
        &candidate.plaintext_evidence_digest,
    )?;
    if expected.same_sha256(&candidate.fingerprint) {
        Ok(())
    } else {
        Err(BackupRestoreError::InvalidCandidateFingerprint)
    }
}

pub fn validate_backup_restore_resource_budget(
    input: BackupRestoreResourceBudgetInput,
) -> Result<(), BackupRestoreError> {
    if input.pack_count > BACKUP_RESTORE_MAX_PACKS {
        return Err(BackupRestoreError::PackCountBudgetExceeded);
    }
    if input.encrypted_bytes > BACKUP_RESTORE_MAX_ENCRYPTED_BYTES {
        return Err(BackupRestoreError::EncryptedBytesBudgetExceeded);
    }
    if input.plaintext_bytes > BACKUP_RESTORE_MAX_PLAINTEXT_BYTES {
        return Err(BackupRestoreError::PlaintextBytesBudgetExceeded);
    }
    Ok(())
}

fn validate_typed_evidence(
    expected_repo_id: RepoId,
    manifest_verification: &BackupVerificationResult,
    plaintext_packs: &BackupPlaintextPacksResult,
) -> Result<(), BackupRestoreError> {
    if manifest_verification.repo_id() != expected_repo_id
        || plaintext_packs.repo_id() != expected_repo_id
    {
        return Err(BackupRestoreError::RepoIdMismatch);
    }
    if !manifest_verification.decrypted() {
        return Err(BackupRestoreError::IncompleteRestoreEvidence);
    }
    if manifest_verification.pack_count() != plaintext_packs.pack_count()
        || manifest_verification.pack_refs().len() != plaintext_packs.pack_refs().len()
    {
        return Err(BackupRestoreError::TypedEvidenceMismatch);
    }
    for (verified, plaintext) in manifest_verification
        .pack_refs()
        .iter()
        .zip(plaintext_packs.pack_refs())
    {
        if verified.pack_sequence() != plaintext.pack_sequence()
            || !verified.digest().same_sha256(plaintext.digest())
        {
            return Err(BackupRestoreError::TypedEvidenceMismatch);
        }
    }
    Ok(())
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

pub(crate) fn plaintext_evidence_digest(
    plaintext_packs: &BackupPlaintextPacksResult,
) -> Result<BackupDigest, BackupRestoreError> {
    let mut transcript = Vec::new();
    transcript.extend_from_slice(b"deve-backup-plaintext-evidence-v1");
    transcript.extend_from_slice(plaintext_packs.repo_id().as_bytes());
    push_str(&mut transcript, plaintext_packs.writer_identity());
    push_str(&mut transcript, plaintext_packs.branch_path());
    push_u64(&mut transcript, plaintext_packs.pack_count());

    for pack in plaintext_packs.plaintext_packs() {
        push_u64(&mut transcript, pack.pack_sequence());
        push_digest(&mut transcript, pack.encrypted_digest())?;
        push_u64(
            &mut transcript,
            u64::try_from(pack.encrypted_bytes()).unwrap_or(u64::MAX),
        );
        push_u64(
            &mut transcript,
            u64::try_from(pack.plaintext_bytes()).unwrap_or(u64::MAX),
        );
        let plaintext = pack.plaintext();
        push_u64(&mut transcript, plaintext.pack_sequence);
        match plaintext.ledger_seq_range {
            Some(range) => {
                transcript.push(1);
                push_u64(&mut transcript, range.start);
                push_u64(&mut transcript, range.end);
            }
            None => transcript.push(0),
        }
        for entry in &plaintext.ledger_entries {
            push_u64(&mut transcript, entry.global_seq);
            push_str(&mut transcript, &sha256_hex(&entry.entry_bytes));
        }
        for snapshot in &plaintext.snapshot_refs {
            push_str(&mut transcript, &snapshot.path);
            push_u64(&mut transcript, snapshot.size_bytes);
            push_digest(&mut transcript, &snapshot.digest)?;
        }
        for blob in &plaintext.blob_refs {
            push_str(&mut transcript, &blob.path);
            push_u64(&mut transcript, blob.size_bytes);
            push_digest(&mut transcript, &blob.digest)?;
        }
    }

    Ok(BackupDigest::sha256(sha256_hex(&transcript)))
}

fn restore_candidate_fingerprint(
    repo_id: RepoId,
    writer_identity: &str,
    branch_path: &str,
    manifest_digest: &BackupDigest,
    pack_count: u64,
    pack_digests: &[BackupDigest],
    plaintext_evidence_digest: &BackupDigest,
) -> Result<BackupDigest, BackupRestoreError> {
    let mut transcript = Vec::new();
    transcript.extend_from_slice(b"deve-restore-candidate-v1");
    transcript.extend_from_slice(repo_id.as_bytes());
    push_str(&mut transcript, writer_identity);
    push_str(&mut transcript, branch_path);
    push_digest(&mut transcript, manifest_digest)?;
    push_u64(&mut transcript, pack_count);
    for digest in pack_digests {
        push_digest(&mut transcript, digest)?;
    }
    push_digest(&mut transcript, plaintext_evidence_digest)?;
    let digest = BackupDigest::sha256(sha256_hex(&transcript));
    if digest.is_valid_sha256() {
        Ok(digest)
    } else {
        Err(BackupRestoreError::InvalidCandidateFingerprint)
    }
}

fn push_u64(transcript: &mut Vec<u8>, value: u64) {
    transcript.extend_from_slice(&value.to_be_bytes());
}

fn push_str(transcript: &mut Vec<u8>, value: &str) {
    push_u64(transcript, u64::try_from(value.len()).unwrap_or(u64::MAX));
    transcript.extend_from_slice(value.as_bytes());
}

fn push_digest(transcript: &mut Vec<u8>, digest: &BackupDigest) -> Result<(), BackupRestoreError> {
    let canonical = digest
        .canonical_sha256_hex()
        .ok_or(BackupRestoreError::InvalidDigest)?;
    push_str(transcript, &canonical);
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
