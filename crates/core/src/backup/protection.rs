//! plan_ref:
//!   - 18_backup#backup-artifact-protection-contract
//!
//! Backup artifact protection admission.
//!
//! This module validates protection metadata before artifacts are allowed to
//! enter upload flow. It does not encrypt, sign, verify signatures, resolve key
//! references, read key material, call providers, or touch local authority.

use super::secret::{BackupSecretRef, BackupSecretRefKind};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupArtifactKind {
    RepoManifest,
    BranchManifest,
    Pack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupProtectionMechanism {
    AeadTag,
    Signature,
    EquivalentIntegrity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupArtifactProtectionInput {
    pub artifact_kind: BackupArtifactKind,
    pub key_ref: BackupSecretRef,
    pub encrypted: bool,
    pub authenticated: bool,
    pub mechanism: BackupProtectionMechanism,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupArtifactProtection {
    pub artifact_kind: BackupArtifactKind,
    pub key_ref: BackupSecretRef,
    pub mechanism: BackupProtectionMechanism,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupArtifactProtectionError {
    #[error("backup artifact protection requires a key ref")]
    KeyRefKindMismatch,
    #[error("backup artifact must be encrypted before upload")]
    ArtifactMustBeEncrypted,
    #[error("backup artifact must be authenticated before upload")]
    ArtifactMustBeAuthenticated,
}

pub fn plan_backup_artifact_protection(
    input: BackupArtifactProtectionInput,
) -> Result<BackupArtifactProtection, BackupArtifactProtectionError> {
    if input.key_ref.kind != BackupSecretRefKind::Key {
        return Err(BackupArtifactProtectionError::KeyRefKindMismatch);
    }
    if !input.encrypted {
        return Err(BackupArtifactProtectionError::ArtifactMustBeEncrypted);
    }
    if !input.authenticated {
        return Err(BackupArtifactProtectionError::ArtifactMustBeAuthenticated);
    }

    Ok(BackupArtifactProtection {
        artifact_kind: input.artifact_kind,
        key_ref: input.key_ref,
        mechanism: input.mechanism,
    })
}
