//! plan_ref:
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-artifact-protection-contract
//!
//! Branch manifest artifact error surface.

use thiserror::Error;

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
    BranchManifest(#[from] super::super::BackupBranchManifestError),
    #[error(transparent)]
    Locator(#[from] super::super::super::BackupLocatorError),
}
