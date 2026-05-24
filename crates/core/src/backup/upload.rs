//! plan_ref:
//!   - 18_backup#backup-upload-state-machine-contract
//!
//! Backup upload state admission.
//!
//! This module validates upload-state metadata after a writable branch binding
//! and pack manifest exist. It does not encrypt, upload, verify remote storage,
//! write ledger state, modify source-control state, or touch Projection
//! Workspaces.

use super::binding::{BackupBindingAccess, BackupBranchBinding};
use super::pack::{BackupDigest, BackupPackError, BackupPackManifest, validate_pack_manifest};
use super::protection::{BackupArtifactKind, BackupArtifactProtection};
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupUploadState {
    PackPlanned,
    PackEncrypted,
    Uploaded,
    RemoteVerified,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupUploadEvidence {
    pub pack_encrypted: bool,
    pub uploaded_payload_digest: Option<BackupDigest>,
    pub remote_manifest_payload_digest: Option<BackupDigest>,
    pub completion_recorded: bool,
}

impl BackupUploadEvidence {
    pub fn none() -> Self {
        Self {
            pack_encrypted: false,
            uploaded_payload_digest: None,
            remote_manifest_payload_digest: None,
            completion_recorded: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupUploadPlanInput {
    pub binding: BackupBranchBinding,
    pub manifest: BackupPackManifest,
    pub protection: Option<BackupArtifactProtection>,
    pub evidence: BackupUploadEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupUploadPlan {
    pub repo_id: RepoId,
    pub branch_name: String,
    pub writer_identity: String,
    pub branch_path: String,
    pub pack_object_path: String,
    pub payload_digest: BackupDigest,
    pub state: BackupUploadState,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupUploadError {
    #[error("backup upload requires a writable branch binding")]
    ReadonlyBindingCannotUpload,
    #[error("backup upload evidence is out of state-machine order")]
    EvidenceOutOfOrder,
    #[error("backup upload digest must be sha256 hex")]
    InvalidDigest,
    #[error("uploaded backup pack digest does not match planned payload")]
    UploadedPackDigestMismatch,
    #[error("remote backup manifest digest does not match uploaded pack")]
    RemoteManifestDigestMismatch,
    #[error("backup upload requires pack protection evidence after encryption")]
    MissingArtifactProtection,
    #[error("backup upload protection evidence must describe a pack artifact")]
    ProtectionKindMismatch,
    #[error(transparent)]
    Pack(#[from] BackupPackError),
}

pub fn plan_backup_upload(
    input: BackupUploadPlanInput,
) -> Result<BackupUploadPlan, BackupUploadError> {
    validate_upload_binding(&input.binding)?;
    validate_pack_manifest(
        &input.manifest,
        input.binding.repo_id,
        &input.binding.writer_identity,
        &input.binding.branch_path,
    )?;
    let state = validate_upload_evidence(
        &input.evidence,
        &input.manifest.payload_digest,
        input.protection.as_ref(),
    )?;

    Ok(BackupUploadPlan {
        repo_id: input.binding.repo_id,
        branch_name: input.binding.branch_name,
        writer_identity: input.binding.writer_identity,
        branch_path: input.binding.branch_path,
        pack_object_path: input.manifest.pack_object_path(),
        payload_digest: input.manifest.payload_digest,
        state,
    })
}

fn validate_upload_binding(binding: &BackupBranchBinding) -> Result<(), BackupUploadError> {
    if binding.access != BackupBindingAccess::Writable {
        return Err(BackupUploadError::ReadonlyBindingCannotUpload);
    }
    Ok(())
}

fn validate_upload_evidence(
    evidence: &BackupUploadEvidence,
    expected_payload_digest: &BackupDigest,
    protection: Option<&BackupArtifactProtection>,
) -> Result<BackupUploadState, BackupUploadError> {
    if !expected_payload_digest.is_valid_sha256() {
        return Err(BackupUploadError::InvalidDigest);
    }

    if (evidence.uploaded_payload_digest.is_some() && !evidence.pack_encrypted)
        || (evidence.remote_manifest_payload_digest.is_some()
            && evidence.uploaded_payload_digest.is_none())
        || (evidence.completion_recorded && evidence.remote_manifest_payload_digest.is_none())
    {
        return Err(BackupUploadError::EvidenceOutOfOrder);
    }

    if let Some(protection) = protection
        && protection.artifact_kind() != BackupArtifactKind::Pack
    {
        return Err(BackupUploadError::ProtectionKindMismatch);
    }
    if evidence.pack_encrypted && protection.is_none() {
        return Err(BackupUploadError::MissingArtifactProtection);
    }

    if let Some(uploaded_digest) = &evidence.uploaded_payload_digest {
        validate_digest(uploaded_digest)?;
        if uploaded_digest != expected_payload_digest {
            return Err(BackupUploadError::UploadedPackDigestMismatch);
        }
    }

    if let Some(remote_digest) = &evidence.remote_manifest_payload_digest {
        validate_digest(remote_digest)?;
        if remote_digest != expected_payload_digest {
            return Err(BackupUploadError::RemoteManifestDigestMismatch);
        }
    }

    Ok(
        match (
            evidence.pack_encrypted,
            evidence.uploaded_payload_digest.is_some(),
            evidence.remote_manifest_payload_digest.is_some(),
            evidence.completion_recorded,
        ) {
            (_, _, _, true) => BackupUploadState::Complete,
            (_, _, true, false) => BackupUploadState::RemoteVerified,
            (_, true, false, false) => BackupUploadState::Uploaded,
            (true, false, false, false) => BackupUploadState::PackEncrypted,
            (false, false, false, false) => BackupUploadState::PackPlanned,
        },
    )
}

fn validate_digest(digest: &BackupDigest) -> Result<(), BackupUploadError> {
    if digest.is_valid_sha256() {
        Ok(())
    } else {
        Err(BackupUploadError::InvalidDigest)
    }
}
