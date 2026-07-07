//! plan_ref:
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-artifact-protection-contract
//!
//! Backup pack artifact error surface.

use super::super::locator::BackupLocatorError;
use super::super::pack::BackupPackError;
use thiserror::Error;

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
    Locator(#[from] BackupLocatorError),
    #[error(transparent)]
    Pack(#[from] BackupPackError),
}
