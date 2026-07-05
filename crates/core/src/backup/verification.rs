//! plan_ref:
//!   - 06_backup#backup-verification-contract
//!
//! Backup artifact verification admission.
//!
//! This module validates verification evidence after remote download and before
//! decrypt/import effects are exposed. It does not perform cryptography, fetch
//! provider objects, append ledger entries, stage source-control changes, or
//! touch Projection Workspaces.

use super::pack::BackupDigest;
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

mod downloaded_packs;
#[cfg(test)]
mod tests;

pub use downloaded_packs::{
    BackupDownloadedPacksInput, BackupDownloadedPacksResult, verify_downloaded_backup_packs,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupPackVerificationEvidence {
    pub pack_sequence: u64,
    pub expected_digest: BackupDigest,
    pub computed_digest: BackupDigest,
    pub authenticated: bool,
    pub decrypted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupVerificationInput {
    pub expected_repo_id: RepoId,
    pub manifest_repo_id: RepoId,
    pub expected_manifest_digest: BackupDigest,
    pub computed_manifest_digest: BackupDigest,
    pub manifest_authenticated: bool,
    pub packs: Vec<BackupPackVerificationEvidence>,
    pub decrypt_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupVerificationResult {
    pub repo_id: RepoId,
    pub manifest_digest: BackupDigest,
    pub pack_count: u64,
    pub pack_digests: Vec<BackupDigest>,
    pub decrypted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupVerificationError {
    #[error("backup verification repo id does not match expected repo")]
    RepoIdMismatch,
    #[error("backup verification digest must be sha256 hex")]
    InvalidDigest,
    #[error("backup verification manifest hash mismatch")]
    ManifestHashMismatch,
    #[error("backup verification manifest authentication failed")]
    ManifestAuthenticationFailed,
    #[error("backup verification pack list is empty")]
    EmptyPackList,
    #[error("backup verification pack sequence must be greater than zero")]
    InvalidPackSequence,
    #[error("backup verification pack sequence is duplicated")]
    DuplicatePackSequence,
    #[error("backup verification downloaded pack object path is duplicated")]
    DuplicatePackObjectPath,
    #[error("backup verification downloaded pack object path mismatch")]
    PackObjectPathMismatch,
    #[error("backup verification downloaded pack is missing")]
    MissingDownloadedPack,
    #[error("backup verification downloaded pack is unexpected")]
    UnexpectedDownloadedPack,
    #[error("backup verification pack hash mismatch")]
    PackHashMismatch,
    #[error("backup verification pack authentication failed")]
    PackAuthenticationFailed,
    #[error("backup verification decrypt was attempted before verification completed")]
    DecryptBeforeVerifyForbidden,
    #[error("backup verification decrypt evidence is required")]
    DecryptFailure,
}

pub fn verify_backup_artifacts(
    input: BackupVerificationInput,
) -> Result<BackupVerificationResult, BackupVerificationError> {
    if input.manifest_repo_id != input.expected_repo_id {
        return Err(BackupVerificationError::RepoIdMismatch);
    }
    validate_digest(&input.expected_manifest_digest)?;
    validate_digest(&input.computed_manifest_digest)?;
    if !input
        .expected_manifest_digest
        .same_sha256(&input.computed_manifest_digest)
    {
        return Err(BackupVerificationError::ManifestHashMismatch);
    }
    if !input.manifest_authenticated {
        return Err(BackupVerificationError::ManifestAuthenticationFailed);
    }
    if input.packs.is_empty() {
        return Err(BackupVerificationError::EmptyPackList);
    }

    let mut pack_digests = Vec::with_capacity(input.packs.len());
    let mut all_decrypted = true;
    let mut pack_sequences = HashSet::with_capacity(input.packs.len());
    for pack in input.packs {
        if !pack_sequences.insert(pack.pack_sequence) {
            return Err(BackupVerificationError::DuplicatePackSequence);
        }
        verify_pack(&pack)?;
        all_decrypted &= pack.decrypted;
        pack_digests.push(pack.expected_digest);
    }

    if input.decrypt_required && !all_decrypted {
        return Err(BackupVerificationError::DecryptFailure);
    }

    Ok(BackupVerificationResult {
        repo_id: input.expected_repo_id,
        manifest_digest: input.expected_manifest_digest,
        pack_count: u64::try_from(pack_digests.len()).unwrap_or(u64::MAX),
        pack_digests,
        decrypted: all_decrypted,
    })
}

fn verify_pack(pack: &BackupPackVerificationEvidence) -> Result<(), BackupVerificationError> {
    if pack.pack_sequence == 0 {
        return Err(BackupVerificationError::InvalidPackSequence);
    }
    validate_digest(&pack.expected_digest)?;
    validate_digest(&pack.computed_digest)?;

    let hash_matches = pack.expected_digest.same_sha256(&pack.computed_digest);
    if pack.decrypted && (!hash_matches || !pack.authenticated) {
        return Err(BackupVerificationError::DecryptBeforeVerifyForbidden);
    }
    if !hash_matches {
        return Err(BackupVerificationError::PackHashMismatch);
    }
    if !pack.authenticated {
        return Err(BackupVerificationError::PackAuthenticationFailed);
    }
    Ok(())
}

fn validate_digest(digest: &BackupDigest) -> Result<(), BackupVerificationError> {
    if digest.is_valid_sha256() {
        Ok(())
    } else {
        Err(BackupVerificationError::InvalidDigest)
    }
}
