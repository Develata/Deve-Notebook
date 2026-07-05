//! plan_ref:
//!   - 06_backup#backup-pack-plaintext-schema-contract
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-verification-contract
//!
//! Pack plaintext schema verification gate.
//!
//! This module validates decrypted pack plaintext bytes against pack metadata
//! carried by the already verified branch manifest. It does not append ledger
//! entries, stage source-control changes, import branches, merge branches, or
//! touch Projection Workspaces.

use super::{
    BackupDecryptedPacksResult, BackupVerificationError, BackupVerifiedPackRef, validate_digest,
};
use crate::backup::{
    BackupBranchManifest, BackupDigest, BackupPackPlaintext, BackupPackPlaintextOpenInput,
    open_backup_pack_plaintext,
};
use crate::models::RepoId;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct BackupPlaintextPacksInput<'a> {
    pub branch_manifest: &'a BackupBranchManifest,
    pub decrypted_packs: &'a BackupDecryptedPacksResult,
}

#[derive(PartialEq, Eq)]
pub struct BackupPlaintextPackPayload {
    pack_sequence: u64,
    object_path: String,
    encrypted_digest: BackupDigest,
    encrypted_bytes: usize,
    plaintext_bytes: usize,
    plaintext: BackupPackPlaintext,
}

impl fmt::Debug for BackupPlaintextPackPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BackupPlaintextPackPayload")
            .field("pack_sequence", &self.pack_sequence)
            .field("object_path", &self.object_path)
            .field("encrypted_digest", &self.encrypted_digest)
            .field("encrypted_bytes", &self.encrypted_bytes)
            .field("plaintext_bytes", &self.plaintext_bytes)
            .finish()
    }
}

impl BackupPlaintextPackPayload {
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

    pub fn plaintext_bytes(&self) -> usize {
        self.plaintext_bytes
    }

    pub fn plaintext(&self) -> &BackupPackPlaintext {
        &self.plaintext
    }
}

#[derive(PartialEq, Eq)]
pub struct BackupPlaintextPacksResult {
    repo_id: RepoId,
    writer_identity: String,
    branch_path: String,
    pack_count: u64,
    pack_refs: Vec<BackupVerifiedPackRef>,
    pack_digests: Vec<BackupDigest>,
    encrypted_bytes_total: usize,
    plaintext_bytes_total: usize,
    plaintext_packs: Vec<BackupPlaintextPackPayload>,
}

impl fmt::Debug for BackupPlaintextPacksResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BackupPlaintextPacksResult")
            .field("repo_id", &self.repo_id)
            .field("writer_identity", &self.writer_identity)
            .field("branch_path", &self.branch_path)
            .field("pack_count", &self.pack_count)
            .field("pack_refs", &self.pack_refs)
            .field("pack_digests", &self.pack_digests)
            .field("encrypted_bytes_total", &self.encrypted_bytes_total)
            .field("plaintext_bytes_total", &self.plaintext_bytes_total)
            .field("plaintext_pack_count", &self.plaintext_packs.len())
            .finish()
    }
}

impl BackupPlaintextPacksResult {
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

    pub fn pack_refs(&self) -> &[BackupVerifiedPackRef] {
        &self.pack_refs
    }

    pub fn encrypted_bytes_total(&self) -> usize {
        self.encrypted_bytes_total
    }

    pub fn plaintext_bytes_total(&self) -> usize {
        self.plaintext_bytes_total
    }

    pub fn plaintext_packs(&self) -> &[BackupPlaintextPackPayload] {
        &self.plaintext_packs
    }
}

pub fn verify_plaintext_backup_packs(
    input: BackupPlaintextPacksInput<'_>,
) -> Result<BackupPlaintextPacksResult, BackupVerificationError> {
    if input.branch_manifest.packs.is_empty() || input.decrypted_packs.plaintext_packs().is_empty()
    {
        return Err(BackupVerificationError::EmptyPackList);
    }
    if input.branch_manifest.repo_id != input.decrypted_packs.repo_id()
        || input.branch_manifest.writer_identity != input.decrypted_packs.writer_identity()
        || input.branch_manifest.branch_path != input.decrypted_packs.branch_path()
    {
        return Err(BackupVerificationError::PackObjectPathMismatch);
    }

    let expected_by_sequence = expected_branch_pack_refs(input.branch_manifest)?;
    let mut seen_sequences = HashSet::with_capacity(input.decrypted_packs.plaintext_packs().len());
    let mut plaintext_packs = Vec::with_capacity(input.decrypted_packs.plaintext_packs().len());

    for pack in input.decrypted_packs.plaintext_packs() {
        if pack.pack_sequence() == 0 {
            return Err(BackupVerificationError::InvalidPackSequence);
        }
        validate_digest(pack.encrypted_digest())?;
        if !seen_sequences.insert(pack.pack_sequence()) {
            return Err(BackupVerificationError::DuplicatePackSequence);
        }

        let expected = expected_by_sequence
            .get(&pack.pack_sequence())
            .ok_or(BackupVerificationError::UnexpectedDecryptedPack)?;
        if pack.object_path() != expected.object_path {
            return Err(BackupVerificationError::PackObjectPathMismatch);
        }
        if !expected.payload_digest.same_sha256(pack.encrypted_digest()) {
            return Err(BackupVerificationError::PackHashMismatch);
        }

        let pack_manifest = input.branch_manifest.pack_manifest_for_ref(expected)?;
        let plaintext = open_backup_pack_plaintext(BackupPackPlaintextOpenInput {
            manifest: &pack_manifest,
            plaintext_bytes: pack.plaintext(),
        })?;
        plaintext_packs.push(BackupPlaintextPackPayload {
            pack_sequence: pack.pack_sequence(),
            object_path: pack.object_path().to_owned(),
            encrypted_digest: pack.encrypted_digest().clone(),
            encrypted_bytes: pack.encrypted_bytes(),
            plaintext_bytes: pack.plaintext().len(),
            plaintext,
        });
    }

    if seen_sequences.len() != expected_by_sequence.len() {
        return Err(BackupVerificationError::MissingDecryptedPack);
    }

    let pack_refs = input
        .decrypted_packs
        .plaintext_packs()
        .iter()
        .map(|pack| BackupVerifiedPackRef {
            pack_sequence: pack.pack_sequence(),
            digest: pack.encrypted_digest().clone(),
        })
        .collect::<Vec<_>>();
    Ok(BackupPlaintextPacksResult {
        repo_id: input.decrypted_packs.repo_id(),
        writer_identity: input.decrypted_packs.writer_identity().to_owned(),
        branch_path: input.decrypted_packs.branch_path().to_owned(),
        pack_count: input.decrypted_packs.pack_count(),
        pack_refs,
        pack_digests: input.decrypted_packs.pack_digests().to_vec(),
        encrypted_bytes_total: input.decrypted_packs.encrypted_bytes_total(),
        plaintext_bytes_total: input.decrypted_packs.plaintext_bytes_total(),
        plaintext_packs,
    })
}

fn expected_branch_pack_refs(
    branch_manifest: &BackupBranchManifest,
) -> Result<HashMap<u64, &crate::backup::BackupBranchManifestPackRef>, BackupVerificationError> {
    let mut expected = HashMap::with_capacity(branch_manifest.packs.len());
    let mut object_paths = HashSet::with_capacity(branch_manifest.packs.len());
    for pack in &branch_manifest.packs {
        if pack.pack_sequence == 0 {
            return Err(BackupVerificationError::InvalidPackSequence);
        }
        validate_digest(&pack.payload_digest)?;
        if !object_paths.insert(pack.object_path.as_str()) {
            return Err(BackupVerificationError::DuplicatePackObjectPath);
        }
        if expected.insert(pack.pack_sequence, pack).is_some() {
            return Err(BackupVerificationError::DuplicatePackSequence);
        }
    }
    Ok(expected)
}

#[cfg(test)]
mod tests;
