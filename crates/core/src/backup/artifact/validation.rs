//! plan_ref:
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-artifact-protection-contract
//!
//! Backup pack artifact protection and routing validation helpers.

use super::super::branch_manifest::{BackupBranchManifest, BackupBranchManifestPackRef};
use super::super::pack::{BackupDigest, BackupPackManifest, validate_pack_manifest};
use super::super::protection::{
    BackupArtifactKind, BackupArtifactProtection, BackupProtectionMechanism,
};
use super::error::BackupPackArtifactError;
use super::types::BackupEncryptedPackArtifact;
use super::{AES_GCM_NONCE_LEN, BACKUP_PACK_ARTIFACT_FORMAT_VERSION, sha256_digest};

pub(super) fn verified_pack_artifact_for_manifest(
    manifest: &BackupPackManifest,
    artifact_bytes: &[u8],
) -> Result<(BackupEncryptedPackArtifact, BackupDigest), BackupPackArtifactError> {
    let artifact: BackupEncryptedPackArtifact = serde_json::from_slice(artifact_bytes)
        .map_err(|_| BackupPackArtifactError::DeserializeFailed)?;
    artifact.validate_against_manifest(manifest)?;
    let computed_digest = sha256_digest(artifact_bytes);
    if !computed_digest.same_sha256(&manifest.payload_digest) {
        return Err(BackupPackArtifactError::ArtifactDigestMismatch);
    }
    artifact.validate_cipher_envelope()?;
    Ok((artifact, computed_digest))
}

pub(super) fn verified_pack_artifact_for_branch_manifest_ref(
    branch_manifest: &BackupBranchManifest,
    pack_ref: &BackupBranchManifestPackRef,
    artifact_bytes: &[u8],
) -> Result<(BackupEncryptedPackArtifact, BackupDigest), BackupPackArtifactError> {
    if !pack_ref.payload_digest.is_valid_sha256() {
        return Err(BackupPackArtifactError::ArtifactDigestMismatch);
    }
    let computed_digest = sha256_digest(artifact_bytes);
    if !computed_digest.same_sha256(&pack_ref.payload_digest) {
        return Err(BackupPackArtifactError::ArtifactDigestMismatch);
    }

    let artifact: BackupEncryptedPackArtifact = serde_json::from_slice(artifact_bytes)
        .map_err(|_| BackupPackArtifactError::DeserializeFailed)?;
    artifact.validate_against_branch_manifest_ref(branch_manifest, pack_ref)?;
    artifact.validate_cipher_envelope()?;
    Ok((artifact, computed_digest))
}

pub(super) fn validate_protection(
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

    fn validate_against_branch_manifest_ref(
        &self,
        branch_manifest: &BackupBranchManifest,
        pack_ref: &BackupBranchManifestPackRef,
    ) -> Result<(), BackupPackArtifactError> {
        if self.format_version != BACKUP_PACK_ARTIFACT_FORMAT_VERSION {
            return Err(BackupPackArtifactError::UnsupportedFormatVersion(
                self.format_version,
            ));
        }
        if self.repo_id != branch_manifest.repo_id {
            return Err(BackupPackArtifactError::RepoIdMismatch);
        }
        if self.writer_identity != branch_manifest.writer_identity {
            return Err(BackupPackArtifactError::WriterIdentityMismatch);
        }
        if self.branch_path != branch_manifest.branch_path {
            return Err(BackupPackArtifactError::BranchPathMismatch);
        }
        if self.pack_sequence != pack_ref.pack_sequence {
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
