//! plan_ref:
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-artifact-protection-contract
//!
//! Encrypted branch manifest artifact sealing and opening.

mod error;

pub use error::BackupBranchManifestArtifactError;

use super::{
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BackupBranchManifest, BackupBranchManifestInput,
    BackupBranchManifestPackRef, validate_backup_branch_manifest,
};
use crate::backup::locator::{BranchBackupLocator, normalize_remote_path, safe_writer_identity};
use crate::backup::{
    BackupArtifactKey, BackupArtifactKind, BackupArtifactProtection, BackupDigest,
    BackupProtectionMechanism,
};
use crate::models::RepoId;
use aes_gcm::{Aes256Gcm, Nonce, aead::Aead, aead::AeadCore, aead::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const BACKUP_BRANCH_MANIFEST_ARTIFACT_FORMAT_VERSION: u32 = 1;
const AES_GCM_NONCE_LEN: usize = 12;

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
