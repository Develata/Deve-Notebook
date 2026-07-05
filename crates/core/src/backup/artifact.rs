//! plan_ref:
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-artifact-protection-contract
//!
//! Encrypted backup pack artifact sealing and opening.
//!
//! This module owns byte-level encryption for backup pack artifacts. It does
//! not resolve key refs, contact providers, write ledger state, modify source
//! control state, or touch Projection Workspaces.

use super::locator::{normalize_remote_path, safe_writer_identity};
use super::pack::{BackupDigest, BackupPackError, BackupPackManifest, validate_pack_manifest};
use super::protection::{BackupArtifactKind, BackupArtifactProtection, BackupProtectionMechanism};
use crate::models::RepoId;
use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[cfg(test)]
mod tests;

const BACKUP_PACK_ARTIFACT_FORMAT_VERSION: u32 = 1;
const AES_GCM_NONCE_LEN: usize = 12;

#[derive(Clone)]
pub struct BackupArtifactKey {
    key_bytes: [u8; 32],
}

impl BackupArtifactKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BackupPackArtifactError> {
        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| BackupPackArtifactError::InvalidKeyLength(bytes.len()))?;
        Ok(Self { key_bytes })
    }

    fn cipher(&self) -> Aes256Gcm {
        let key = Key::<Aes256Gcm>::from_slice(&self.key_bytes);
        Aes256Gcm::new(key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupEncryptedPackArtifact {
    pub format_version: u32,
    pub repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub pack_sequence: u64,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl BackupEncryptedPackArtifact {
    pub fn to_bytes(&self) -> Result<Vec<u8>, BackupPackArtifactError> {
        serde_json::to_vec(self).map_err(|_| BackupPackArtifactError::SerializeFailed)
    }

    pub fn payload_digest(&self) -> Result<BackupDigest, BackupPackArtifactError> {
        Ok(sha256_digest(&self.to_bytes()?))
    }
}

pub struct BackupPackArtifactInput<'a> {
    pub repo_id: RepoId,
    pub writer_identity: &'a str,
    pub branch_path: &'a str,
    pub pack_sequence: u64,
    pub protection: &'a BackupArtifactProtection,
    pub key: &'a BackupArtifactKey,
    pub plaintext: &'a [u8],
}

pub struct BackupPackArtifactOpenInput<'a> {
    pub manifest: &'a BackupPackManifest,
    pub key: &'a BackupArtifactKey,
    pub artifact_bytes: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupPackArtifactError {
    #[error("backup artifact key must be 32 bytes, got {0}")]
    InvalidKeyLength(usize),
    #[error("backup pack artifact plaintext must not be empty")]
    EmptyPlaintext,
    #[error("backup pack artifact sequence must be greater than zero")]
    InvalidPackSequence,
    #[error("backup pack artifact format version is unsupported: {0}")]
    UnsupportedFormatVersion(u32),
    #[error("backup pack artifact protection must describe a pack artifact")]
    ProtectionKindMismatch,
    #[error("backup pack artifact encryption requires AEAD tag protection")]
    UnsupportedProtectionMechanism,
    #[error("backup pack artifact digest does not match manifest")]
    ArtifactDigestMismatch,
    #[error("backup pack artifact repo id does not match manifest")]
    RepoIdMismatch,
    #[error("backup pack artifact writer identity does not match manifest")]
    WriterIdentityMismatch,
    #[error("backup pack artifact branch path does not match manifest")]
    BranchPathMismatch,
    #[error("backup pack artifact sequence does not match manifest")]
    PackSequenceMismatch,
    #[error("backup pack artifact nonce length is invalid")]
    InvalidNonceLength,
    #[error("backup pack artifact ciphertext is empty")]
    EmptyCiphertext,
    #[error("backup pack artifact encryption failed")]
    EncryptFailed,
    #[error("backup pack artifact decryption failed")]
    DecryptFailed,
    #[error("backup pack artifact serialization failed")]
    SerializeFailed,
    #[error("backup pack artifact deserialization failed")]
    DeserializeFailed,
    #[error(transparent)]
    Locator(#[from] super::BackupLocatorError),
    #[error(transparent)]
    Pack(#[from] BackupPackError),
}

pub fn encrypt_backup_pack_artifact(
    input: BackupPackArtifactInput<'_>,
) -> Result<BackupEncryptedPackArtifact, BackupPackArtifactError> {
    validate_protection(input.protection)?;
    if input.plaintext.is_empty() {
        return Err(BackupPackArtifactError::EmptyPlaintext);
    }
    if input.pack_sequence == 0 {
        return Err(BackupPackArtifactError::InvalidPackSequence);
    }

    let writer_identity = safe_writer_identity(input.writer_identity)?;
    let branch_path = normalize_remote_path(input.branch_path)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = input
        .key
        .cipher()
        .encrypt(&nonce, input.plaintext)
        .map_err(|_| BackupPackArtifactError::EncryptFailed)?;

    Ok(BackupEncryptedPackArtifact {
        format_version: BACKUP_PACK_ARTIFACT_FORMAT_VERSION,
        repo_id: input.repo_id,
        writer_identity,
        branch_path,
        pack_sequence: input.pack_sequence,
        nonce: nonce.to_vec(),
        ciphertext,
    })
}

pub fn decrypt_backup_pack_artifact(
    input: BackupPackArtifactOpenInput<'_>,
) -> Result<Vec<u8>, BackupPackArtifactError> {
    let artifact: BackupEncryptedPackArtifact = serde_json::from_slice(input.artifact_bytes)
        .map_err(|_| BackupPackArtifactError::DeserializeFailed)?;
    artifact.validate_against_manifest(input.manifest)?;
    let computed_digest = sha256_digest(input.artifact_bytes);
    if !computed_digest.same_sha256(&input.manifest.payload_digest) {
        return Err(BackupPackArtifactError::ArtifactDigestMismatch);
    }
    artifact.validate_cipher_envelope()?;

    let nonce = Nonce::from_slice(&artifact.nonce);
    input
        .key
        .cipher()
        .decrypt(nonce, artifact.ciphertext.as_ref())
        .map_err(|_| BackupPackArtifactError::DecryptFailed)
}

impl BackupEncryptedPackArtifact {
    fn validate_against_manifest(
        &self,
        manifest: &BackupPackManifest,
    ) -> Result<(), BackupPackArtifactError> {
        validate_pack_manifest(
            manifest,
            manifest.repo_id,
            &manifest.writer_identity,
            &manifest.branch_path,
        )?;
        if self.format_version != BACKUP_PACK_ARTIFACT_FORMAT_VERSION {
            return Err(BackupPackArtifactError::UnsupportedFormatVersion(
                self.format_version,
            ));
        }
        if self.repo_id != manifest.repo_id {
            return Err(BackupPackArtifactError::RepoIdMismatch);
        }
        if self.writer_identity != manifest.writer_identity {
            return Err(BackupPackArtifactError::WriterIdentityMismatch);
        }
        if self.branch_path != manifest.branch_path {
            return Err(BackupPackArtifactError::BranchPathMismatch);
        }
        if self.pack_sequence != manifest.pack_sequence {
            return Err(BackupPackArtifactError::PackSequenceMismatch);
        }
        Ok(())
    }

    fn validate_cipher_envelope(&self) -> Result<(), BackupPackArtifactError> {
        if self.nonce.len() != AES_GCM_NONCE_LEN {
            return Err(BackupPackArtifactError::InvalidNonceLength);
        }
        if self.ciphertext.is_empty() {
            return Err(BackupPackArtifactError::EmptyCiphertext);
        }
        Ok(())
    }
}

fn validate_protection(
    protection: &BackupArtifactProtection,
) -> Result<(), BackupPackArtifactError> {
    if protection.artifact_kind() != BackupArtifactKind::Pack {
        return Err(BackupPackArtifactError::ProtectionKindMismatch);
    }
    if protection.mechanism() != BackupProtectionMechanism::AeadTag {
        return Err(BackupPackArtifactError::UnsupportedProtectionMechanism);
    }
    Ok(())
}

fn sha256_digest(bytes: &[u8]) -> BackupDigest {
    BackupDigest::sha256(hex::encode(Sha256::digest(bytes)))
}
