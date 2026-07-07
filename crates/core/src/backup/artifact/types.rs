//! plan_ref:
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-artifact-protection-contract
//!
//! Backup pack artifact typed inputs and evidence.

use super::super::branch_manifest::{BackupBranchManifest, BackupBranchManifestPackRef};
use super::super::pack::{BackupDigest, BackupPackManifest};
use super::super::protection::BackupArtifactProtection;
use super::error::BackupPackArtifactError;
use super::sha256_digest;
use crate::models::RepoId;
use aes_gcm::{Aes256Gcm, Key, KeyInit};
use serde::{Deserialize, Serialize};
use std::fmt;

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

    pub(crate) fn cipher(&self) -> Aes256Gcm {
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

pub struct BackupPackArtifactUploadVerifyInput<'a> {
    pub manifest: &'a BackupPackManifest,
    pub artifact_bytes: &'a [u8],
}

pub struct BackupPackArtifactDownloadVerifyInput<'a> {
    pub manifest: &'a BackupPackManifest,
    pub artifact_bytes: &'a [u8],
}

pub struct BackupPackArtifactRefDownloadVerifyInput<'a> {
    pub branch_manifest: &'a BackupBranchManifest,
    pub pack_ref: &'a BackupBranchManifestPackRef,
    pub artifact_bytes: &'a [u8],
}

pub struct BackupPackArtifactRefOpenInput<'a> {
    pub branch_manifest: &'a BackupBranchManifest,
    pub pack_ref: &'a BackupBranchManifestPackRef,
    pub key: &'a BackupArtifactKey,
    pub artifact_bytes: &'a [u8],
}

/// Verified and decrypted pack artifact bytes.
///
/// This result is produced only after manifest/routing/digest verification and
/// AEAD decrypt. It carries plaintext for restore assembly, but no credential
/// or key material.
#[derive(PartialEq, Eq)]
pub struct BackupPackArtifactOpenResult {
    pub(super) pack_sequence: u64,
    pub(super) object_path: String,
    pub(super) computed_digest: BackupDigest,
    pub(super) encrypted_bytes: usize,
    pub(super) plaintext: Vec<u8>,
}

impl fmt::Debug for BackupPackArtifactOpenResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BackupPackArtifactOpenResult")
            .field("pack_sequence", &self.pack_sequence)
            .field("object_path", &self.object_path)
            .field("computed_digest", &self.computed_digest)
            .field("encrypted_bytes", &self.encrypted_bytes)
            .field("plaintext_len", &self.plaintext.len())
            .finish()
    }
}

impl BackupPackArtifactOpenResult {
    pub fn pack_sequence(&self) -> u64 {
        self.pack_sequence
    }

    pub fn object_path(&self) -> &str {
        &self.object_path
    }

    pub fn computed_digest(&self) -> &BackupDigest {
        &self.computed_digest
    }

    pub fn encrypted_bytes(&self) -> usize {
        self.encrypted_bytes
    }

    pub fn plaintext(&self) -> &[u8] {
        &self.plaintext
    }

    pub fn into_plaintext(self) -> Vec<u8> {
        self.plaintext
    }
}

/// Digest/routing evidence for downloaded encrypted pack bytes.
///
/// This is not AEAD authentication, not decrypt, and not restore candidate
/// admission. It intentionally exposes no plaintext.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPackArtifactDownloadVerifyResult {
    pub(super) pack_sequence: u64,
    pub(super) object_path: String,
    pub(super) computed_digest: BackupDigest,
}

impl BackupPackArtifactDownloadVerifyResult {
    pub fn pack_sequence(&self) -> u64 {
        self.pack_sequence
    }

    pub fn object_path(&self) -> &str {
        &self.object_path
    }

    pub fn computed_digest(&self) -> &BackupDigest {
        &self.computed_digest
    }
}
