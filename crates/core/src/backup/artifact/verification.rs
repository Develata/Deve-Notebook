//! plan_ref:
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-artifact-protection-contract
//!
//! Backup pack artifact encryption, opening, and digest/routing verification.

use super::super::locator::{normalize_remote_path, safe_writer_identity};
use super::super::pack::BackupDigest;
use super::BACKUP_PACK_ARTIFACT_FORMAT_VERSION;
use super::error::BackupPackArtifactError;
use super::types::{
    BackupEncryptedPackArtifact, BackupPackArtifactDownloadVerifyInput,
    BackupPackArtifactDownloadVerifyResult, BackupPackArtifactInput, BackupPackArtifactOpenInput,
    BackupPackArtifactOpenResult, BackupPackArtifactRefDownloadVerifyInput,
    BackupPackArtifactRefOpenInput, BackupPackArtifactUploadVerifyInput,
};
use super::validation::{
    validate_protection, verified_pack_artifact_for_branch_manifest_ref,
    verified_pack_artifact_for_manifest,
};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, OsRng},
};

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
    Ok(open_backup_pack_artifact(input)?.into_plaintext())
}

pub fn open_backup_pack_artifact(
    input: BackupPackArtifactOpenInput<'_>,
) -> Result<BackupPackArtifactOpenResult, BackupPackArtifactError> {
    let (artifact, computed_digest) =
        verified_pack_artifact_for_manifest(input.manifest, input.artifact_bytes)?;
    let nonce = Nonce::from_slice(&artifact.nonce);
    let plaintext = input
        .key
        .cipher()
        .decrypt(nonce, artifact.ciphertext.as_ref())
        .map_err(|_| BackupPackArtifactError::DecryptFailed)?;
    if plaintext.is_empty() {
        return Err(BackupPackArtifactError::EmptyPlaintext);
    }

    Ok(BackupPackArtifactOpenResult {
        pack_sequence: artifact.pack_sequence,
        object_path: input.manifest.pack_object_path(),
        computed_digest,
        encrypted_bytes: input.artifact_bytes.len(),
        plaintext,
    })
}

pub fn open_backup_pack_artifact_ref(
    input: BackupPackArtifactRefOpenInput<'_>,
) -> Result<BackupPackArtifactOpenResult, BackupPackArtifactError> {
    let (artifact, computed_digest) = verified_pack_artifact_for_branch_manifest_ref(
        input.branch_manifest,
        input.pack_ref,
        input.artifact_bytes,
    )?;
    let nonce = Nonce::from_slice(&artifact.nonce);
    let plaintext = input
        .key
        .cipher()
        .decrypt(nonce, artifact.ciphertext.as_ref())
        .map_err(|_| BackupPackArtifactError::DecryptFailed)?;
    if plaintext.is_empty() {
        return Err(BackupPackArtifactError::EmptyPlaintext);
    }

    Ok(BackupPackArtifactOpenResult {
        pack_sequence: artifact.pack_sequence,
        object_path: input.pack_ref.object_path.clone(),
        computed_digest,
        encrypted_bytes: input.artifact_bytes.len(),
        plaintext,
    })
}

pub fn verify_backup_pack_artifact_for_upload(
    input: BackupPackArtifactUploadVerifyInput<'_>,
) -> Result<BackupDigest, BackupPackArtifactError> {
    let (_, computed_digest) =
        verified_pack_artifact_for_manifest(input.manifest, input.artifact_bytes)?;
    Ok(computed_digest)
}

pub fn verify_downloaded_pack_artifact_digest_and_routing(
    input: BackupPackArtifactDownloadVerifyInput<'_>,
) -> Result<BackupPackArtifactDownloadVerifyResult, BackupPackArtifactError> {
    let (artifact, computed_digest) =
        verified_pack_artifact_for_manifest(input.manifest, input.artifact_bytes)?;
    Ok(BackupPackArtifactDownloadVerifyResult {
        pack_sequence: artifact.pack_sequence,
        object_path: input.manifest.pack_object_path(),
        computed_digest,
    })
}

pub fn verify_downloaded_pack_artifact_ref_and_routing(
    input: BackupPackArtifactRefDownloadVerifyInput<'_>,
) -> Result<BackupPackArtifactDownloadVerifyResult, BackupPackArtifactError> {
    let (artifact, computed_digest) = verified_pack_artifact_for_branch_manifest_ref(
        input.branch_manifest,
        input.pack_ref,
        input.artifact_bytes,
    )?;
    Ok(BackupPackArtifactDownloadVerifyResult {
        pack_sequence: artifact.pack_sequence,
        object_path: input.pack_ref.object_path.clone(),
        computed_digest,
    })
}
