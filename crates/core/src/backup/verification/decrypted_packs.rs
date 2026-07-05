//! plan_ref:
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-verification-contract
//!
//! Decrypted pack evidence gate.
//!
//! This module checks that decrypted pack artifact results correspond exactly
//! to the already verified PacksDownloaded set. It does not fetch providers,
//! resolve keys, append ledger entries, stage source-control changes, import
//! branches, merge branches, or touch Projection Workspaces.

use super::{
    BackupDownloadedPackRef, BackupDownloadedPacksResult, BackupVerificationError, validate_digest,
};
use crate::backup::{BackupDigest, BackupPackArtifactOpenResult};
use crate::models::RepoId;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct BackupDecryptedPacksInput<'a> {
    pub downloaded_packs: &'a BackupDownloadedPacksResult,
    pub opened_packs: Vec<BackupPackArtifactOpenResult>,
}

#[derive(PartialEq, Eq)]
pub struct BackupDecryptedPackPayload {
    pack_sequence: u64,
    object_path: String,
    encrypted_digest: BackupDigest,
    encrypted_bytes: usize,
    plaintext: Vec<u8>,
}

impl fmt::Debug for BackupDecryptedPackPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BackupDecryptedPackPayload")
            .field("pack_sequence", &self.pack_sequence)
            .field("object_path", &self.object_path)
            .field("encrypted_digest", &self.encrypted_digest)
            .field("encrypted_bytes", &self.encrypted_bytes)
            .field("plaintext_len", &self.plaintext.len())
            .finish()
    }
}

impl BackupDecryptedPackPayload {
    pub fn pack_sequence(&self) -> u64 {
        self.pack_sequence
    }

    pub fn object_path(&self) -> &str {
        &self.object_path
    }

    pub fn encrypted_digest(&self) -> &BackupDigest {
        &self.encrypted_digest
    }

    pub fn encrypted_bytes(&self) -> usize {
        self.encrypted_bytes
    }

    pub fn plaintext(&self) -> &[u8] {
        &self.plaintext
    }
}

#[derive(PartialEq, Eq)]
pub struct BackupDecryptedPacksResult {
    repo_id: RepoId,
    writer_identity: String,
    branch_path: String,
    pack_count: u64,
    pack_digests: Vec<BackupDigest>,
    encrypted_bytes_total: usize,
    plaintext_bytes_total: usize,
    plaintext_packs: Vec<BackupDecryptedPackPayload>,
}

impl fmt::Debug for BackupDecryptedPacksResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BackupDecryptedPacksResult")
            .field("repo_id", &self.repo_id)
            .field("writer_identity", &self.writer_identity)
            .field("branch_path", &self.branch_path)
            .field("pack_count", &self.pack_count)
            .field("pack_digests", &self.pack_digests)
            .field("encrypted_bytes_total", &self.encrypted_bytes_total)
            .field("plaintext_bytes_total", &self.plaintext_bytes_total)
            .field("plaintext_pack_count", &self.plaintext_packs.len())
            .finish()
    }
}

impl BackupDecryptedPacksResult {
    pub fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub fn writer_identity(&self) -> &str {
        &self.writer_identity
    }

    pub fn branch_path(&self) -> &str {
        &self.branch_path
    }

    pub fn pack_count(&self) -> u64 {
        self.pack_count
    }

    pub fn pack_digests(&self) -> &[BackupDigest] {
        &self.pack_digests
    }

    pub fn encrypted_bytes_total(&self) -> usize {
        self.encrypted_bytes_total
    }

    pub fn plaintext_bytes_total(&self) -> usize {
        self.plaintext_bytes_total
    }

    pub fn plaintext_packs(&self) -> &[BackupDecryptedPackPayload] {
        &self.plaintext_packs
    }
}

pub fn verify_decrypted_backup_packs(
    input: BackupDecryptedPacksInput<'_>,
) -> Result<BackupDecryptedPacksResult, BackupVerificationError> {
    if input.downloaded_packs.pack_refs().is_empty() || input.opened_packs.is_empty() {
        return Err(BackupVerificationError::EmptyPackList);
    }

    let expected_by_sequence = expected_downloaded_refs(input.downloaded_packs.pack_refs())?;
    let mut seen_sequences = HashSet::with_capacity(input.opened_packs.len());
    let mut seen_paths = HashSet::with_capacity(input.opened_packs.len());
    let mut opened_by_sequence = HashMap::with_capacity(input.opened_packs.len());

    for pack in input.opened_packs {
        if pack.pack_sequence() == 0 {
            return Err(BackupVerificationError::InvalidPackSequence);
        }
        validate_digest(pack.computed_digest())?;
        if pack.plaintext().is_empty() {
            return Err(BackupVerificationError::EmptyDecryptedPack);
        }
        if !seen_sequences.insert(pack.pack_sequence()) {
            return Err(BackupVerificationError::DuplicatePackSequence);
        }
        if !seen_paths.insert(pack.object_path().to_owned()) {
            return Err(BackupVerificationError::DuplicatePackObjectPath);
        }

        let expected = expected_by_sequence
            .get(&pack.pack_sequence())
            .ok_or(BackupVerificationError::UnexpectedDecryptedPack)?;
        if pack.object_path() != expected.object_path() {
            return Err(BackupVerificationError::PackObjectPathMismatch);
        }
        if !expected
            .payload_digest()
            .same_sha256(pack.computed_digest())
        {
            return Err(BackupVerificationError::PackHashMismatch);
        }
        opened_by_sequence.insert(pack.pack_sequence(), pack);
    }

    if seen_sequences.len() != expected_by_sequence.len() {
        return Err(BackupVerificationError::MissingDecryptedPack);
    }

    let mut plaintext_packs = Vec::with_capacity(input.downloaded_packs.pack_refs().len());
    let mut encrypted_bytes_total = 0usize;
    let mut plaintext_bytes_total = 0usize;
    for expected in input.downloaded_packs.pack_refs() {
        let pack = opened_by_sequence
            .remove(&expected.pack_sequence())
            .ok_or(BackupVerificationError::MissingDecryptedPack)?;
        let encrypted_bytes = pack.encrypted_bytes();
        let plaintext = pack.into_plaintext();
        encrypted_bytes_total = encrypted_bytes_total.saturating_add(encrypted_bytes);
        plaintext_bytes_total = plaintext_bytes_total.saturating_add(plaintext.len());
        plaintext_packs.push(BackupDecryptedPackPayload {
            pack_sequence: expected.pack_sequence(),
            object_path: expected.object_path().to_owned(),
            encrypted_digest: expected.payload_digest().clone(),
            encrypted_bytes,
            plaintext,
        });
    }

    Ok(BackupDecryptedPacksResult {
        repo_id: input.downloaded_packs.repo_id(),
        writer_identity: input.downloaded_packs.writer_identity().to_owned(),
        branch_path: input.downloaded_packs.branch_path().to_owned(),
        pack_count: input.downloaded_packs.pack_count(),
        pack_digests: input.downloaded_packs.pack_digests().to_vec(),
        encrypted_bytes_total,
        plaintext_bytes_total,
        plaintext_packs,
    })
}

fn expected_downloaded_refs(
    refs: &[BackupDownloadedPackRef],
) -> Result<HashMap<u64, &BackupDownloadedPackRef>, BackupVerificationError> {
    let mut expected = HashMap::with_capacity(refs.len());
    let mut object_paths = HashSet::with_capacity(refs.len());
    for pack in refs {
        if pack.pack_sequence() == 0 {
            return Err(BackupVerificationError::InvalidPackSequence);
        }
        validate_digest(pack.payload_digest())?;
        if !object_paths.insert(pack.object_path()) {
            return Err(BackupVerificationError::DuplicatePackObjectPath);
        }
        if expected.insert(pack.pack_sequence(), pack).is_some() {
            return Err(BackupVerificationError::DuplicatePackSequence);
        }
    }
    Ok(expected)
}

#[cfg(test)]
mod tests;
