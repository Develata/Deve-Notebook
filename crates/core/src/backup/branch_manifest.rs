//! plan_ref:
//!   - 06_backup#backup-branch-binding-contract
//!   - 06_backup#backup-remote-layout-contract
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-artifact-protection-contract
//!
//! Backup branch manifest validation and encrypted artifact opening.
//!
//! This module validates branch manifest metadata and pack object references
//! and owns byte-level opening for branch manifest artifacts. It does not fetch
//! providers, resolve key refs, append ledger entries, stage source-control
//! changes, or touch Projection Workspaces.

use super::artifact::BackupArtifactKey;
use super::locator::{BranchBackupLocator, normalize_remote_path, safe_writer_identity};
use super::pack::{BackupDigest, pack_file_name};
use super::protection::{BackupArtifactKind, BackupArtifactProtection, BackupProtectionMechanism};
use crate::models::RepoId;
use aes_gcm::{Aes256Gcm, Nonce, aead::Aead, aead::AeadCore, aead::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use thiserror::Error;

#[cfg(test)]
mod tests;

pub const BACKUP_BRANCH_MANIFEST_FORMAT_VERSION: u32 = 1;
const BACKUP_BRANCH_MANIFEST_ARTIFACT_FORMAT_VERSION: u32 = 1;
const AES_GCM_NONCE_LEN: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupEncryptedBranchManifestArtifact {
    pub format_version: u32,
    pub repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl BackupEncryptedBranchManifestArtifact {
    pub fn to_bytes(&self) -> Result<Vec<u8>, BackupBranchManifestArtifactError> {
        serde_json::to_vec(self).map_err(|_| BackupBranchManifestArtifactError::SerializeFailed)
    }

    pub fn payload_digest(&self) -> Result<BackupDigest, BackupBranchManifestArtifactError> {
        Ok(sha256_digest(&self.to_bytes()?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupBranchManifestArtifactPlaintext {
    format_version: u32,
    repo_id: RepoId,
    writer_identity: String,
    branch_path: String,
    packs: Vec<BackupBranchManifestPackRef>,
}

pub struct BackupBranchManifestArtifactInput<'a> {
    pub branch: BranchBackupLocator,
    pub repo_id: RepoId,
    pub writer_identity: &'a str,
    pub branch_path: &'a str,
    pub packs: Vec<BackupBranchManifestPackRef>,
    pub protection: &'a BackupArtifactProtection,
    pub key: &'a BackupArtifactKey,
}

pub struct BackupBranchManifestArtifactOpenInput<'a> {
    pub branch: BranchBackupLocator,
    pub expected_repo_id: RepoId,
    pub expected_manifest_digest: BackupDigest,
    pub key: &'a BackupArtifactKey,
    pub artifact_bytes: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupBranchManifestArtifactOpenResult {
    branch_manifest: BackupBranchManifest,
    computed_digest: BackupDigest,
}

impl BackupBranchManifestArtifactOpenResult {
    pub fn branch_manifest(&self) -> &BackupBranchManifest {
        &self.branch_manifest
    }

    pub fn computed_digest(&self) -> &BackupDigest {
        &self.computed_digest
    }

    pub fn into_branch_manifest(self) -> BackupBranchManifest {
        self.branch_manifest
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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupBranchManifestArtifactError {
    #[error("backup branch manifest artifact format version is unsupported: {0}")]
    UnsupportedFormatVersion(u32),
    #[error("backup branch manifest artifact protection must describe a branch manifest")]
    ProtectionKindMismatch,
    #[error("backup branch manifest artifact encryption requires AEAD tag protection")]
    UnsupportedProtectionMechanism,
    #[error("backup branch manifest artifact digest must be sha256 hex")]
    InvalidDigest,
    #[error("backup branch manifest artifact digest does not match expected manifest digest")]
    ArtifactDigestMismatch,
    #[error("backup branch manifest artifact repo id does not match expected repo")]
    RepoIdMismatch,
    #[error("backup branch manifest artifact writer identity does not match locator writer")]
    WriterIdentityMismatch,
    #[error("backup branch manifest artifact path does not match locator branch path")]
    BranchPathMismatch,
    #[error("backup branch manifest artifact nonce length is invalid")]
    InvalidNonceLength,
    #[error("backup branch manifest artifact ciphertext is empty")]
    EmptyCiphertext,
    #[error("backup branch manifest artifact plaintext is empty")]
    EmptyPlaintext,
    #[error("backup branch manifest artifact encryption failed")]
    EncryptFailed,
    #[error("backup branch manifest artifact decryption failed")]
    DecryptFailed,
    #[error("backup branch manifest artifact serialization failed")]
    SerializeFailed,
    #[error("backup branch manifest artifact deserialization failed")]
    DeserializeFailed,
    #[error(transparent)]
    BranchManifest(#[from] BackupBranchManifestError),
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

pub fn encrypt_backup_branch_manifest_artifact(
    input: BackupBranchManifestArtifactInput<'_>,
) -> Result<BackupEncryptedBranchManifestArtifact, BackupBranchManifestArtifactError> {
    validate_branch_manifest_protection(input.protection)?;
    let branch_manifest = validate_backup_branch_manifest(BackupBranchManifestInput {
        branch: input.branch,
        expected_repo_id: input.repo_id,
        manifest_repo_id: input.repo_id,
        manifest_writer_identity: input.writer_identity.to_string(),
        manifest_branch_path: input.branch_path.to_string(),
        format_version: BACKUP_BRANCH_MANIFEST_FORMAT_VERSION,
        packs: input.packs,
    })?;
    let plaintext = BackupBranchManifestArtifactPlaintext {
        format_version: branch_manifest.format_version,
        repo_id: branch_manifest.repo_id,
        writer_identity: branch_manifest.writer_identity.clone(),
        branch_path: branch_manifest.branch_path.clone(),
        packs: branch_manifest.packs,
    };
    let plaintext = serde_json::to_vec(&plaintext)
        .map_err(|_| BackupBranchManifestArtifactError::SerializeFailed)?;
    if plaintext.is_empty() {
        return Err(BackupBranchManifestArtifactError::EmptyPlaintext);
    }

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher_for_key(input.key)
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(|_| BackupBranchManifestArtifactError::EncryptFailed)?;
    Ok(BackupEncryptedBranchManifestArtifact {
        format_version: BACKUP_BRANCH_MANIFEST_ARTIFACT_FORMAT_VERSION,
        repo_id: input.repo_id,
        writer_identity: safe_writer_identity(input.writer_identity)?,
        branch_path: normalize_remote_path(input.branch_path)?,
        nonce: nonce.to_vec(),
        ciphertext,
    })
}

pub fn open_backup_branch_manifest_artifact(
    input: BackupBranchManifestArtifactOpenInput<'_>,
) -> Result<BackupBranchManifestArtifactOpenResult, BackupBranchManifestArtifactError> {
    validate_digest(&input.expected_manifest_digest)?;
    let computed_digest = sha256_digest(input.artifact_bytes);
    if !computed_digest.same_sha256(&input.expected_manifest_digest) {
        return Err(BackupBranchManifestArtifactError::ArtifactDigestMismatch);
    }

    let artifact: BackupEncryptedBranchManifestArtifact =
        serde_json::from_slice(input.artifact_bytes)
            .map_err(|_| BackupBranchManifestArtifactError::DeserializeFailed)?;
    artifact.validate_envelope(&input.branch, input.expected_repo_id)?;
    let nonce = Nonce::from_slice(&artifact.nonce);
    let plaintext = cipher_for_key(input.key)
        .decrypt(nonce, artifact.ciphertext.as_ref())
        .map_err(|_| BackupBranchManifestArtifactError::DecryptFailed)?;
    if plaintext.is_empty() {
        return Err(BackupBranchManifestArtifactError::EmptyPlaintext);
    }

    let plaintext: BackupBranchManifestArtifactPlaintext = serde_json::from_slice(&plaintext)
        .map_err(|_| BackupBranchManifestArtifactError::DeserializeFailed)?;
    let branch_manifest = validate_backup_branch_manifest(BackupBranchManifestInput {
        branch: input.branch,
        expected_repo_id: input.expected_repo_id,
        manifest_repo_id: plaintext.repo_id,
        manifest_writer_identity: plaintext.writer_identity,
        manifest_branch_path: plaintext.branch_path,
        format_version: plaintext.format_version,
        packs: plaintext.packs,
    })?;
    Ok(BackupBranchManifestArtifactOpenResult {
        branch_manifest,
        computed_digest,
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

impl BackupEncryptedBranchManifestArtifact {
    fn validate_envelope(
        &self,
        branch: &BranchBackupLocator,
        expected_repo_id: RepoId,
    ) -> Result<(), BackupBranchManifestArtifactError> {
        if self.format_version != BACKUP_BRANCH_MANIFEST_ARTIFACT_FORMAT_VERSION {
            return Err(BackupBranchManifestArtifactError::UnsupportedFormatVersion(
                self.format_version,
            ));
        }
        if self.repo_id != expected_repo_id {
            return Err(BackupBranchManifestArtifactError::RepoIdMismatch);
        }
        if self.writer_identity != branch.writer_identity {
            return Err(BackupBranchManifestArtifactError::WriterIdentityMismatch);
        }
        if self.branch_path != branch.branch_path {
            return Err(BackupBranchManifestArtifactError::BranchPathMismatch);
        }
        if self.nonce.len() != AES_GCM_NONCE_LEN {
            return Err(BackupBranchManifestArtifactError::InvalidNonceLength);
        }
        if self.ciphertext.is_empty() {
            return Err(BackupBranchManifestArtifactError::EmptyCiphertext);
        }
        Ok(())
    }
}

fn validate_branch_manifest_protection(
    protection: &BackupArtifactProtection,
) -> Result<(), BackupBranchManifestArtifactError> {
    if protection.artifact_kind() != BackupArtifactKind::BranchManifest {
        return Err(BackupBranchManifestArtifactError::ProtectionKindMismatch);
    }
    if protection.mechanism() != BackupProtectionMechanism::AeadTag {
        return Err(BackupBranchManifestArtifactError::UnsupportedProtectionMechanism);
    }
    Ok(())
}

fn validate_digest(digest: &BackupDigest) -> Result<(), BackupBranchManifestArtifactError> {
    if !digest.is_valid_sha256() {
        return Err(BackupBranchManifestArtifactError::InvalidDigest);
    }
    Ok(())
}

fn cipher_for_key(key: &BackupArtifactKey) -> Aes256Gcm {
    key.cipher()
}

fn sha256_digest(bytes: &[u8]) -> BackupDigest {
    BackupDigest::sha256(hex::encode(Sha256::digest(bytes)))
}
