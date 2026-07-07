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

mod error;
mod types;
mod validation;
mod verification;

#[cfg(test)]
use super::pack::BackupPackManifest;
#[cfg(test)]
use super::protection::BackupArtifactProtection;

#[cfg(test)]
mod tests;

pub use error::BackupPackArtifactError;
pub use types::{
    BackupArtifactKey, BackupEncryptedPackArtifact, BackupPackArtifactDownloadVerifyInput,
    BackupPackArtifactDownloadVerifyResult, BackupPackArtifactInput, BackupPackArtifactOpenInput,
    BackupPackArtifactOpenResult, BackupPackArtifactRefDownloadVerifyInput,
    BackupPackArtifactRefOpenInput, BackupPackArtifactUploadVerifyInput,
};
pub use verification::{
    decrypt_backup_pack_artifact, encrypt_backup_pack_artifact, open_backup_pack_artifact,
    open_backup_pack_artifact_ref, verify_backup_pack_artifact_for_upload,
    verify_downloaded_pack_artifact_digest_and_routing,
    verify_downloaded_pack_artifact_ref_and_routing,
};

pub(super) const BACKUP_PACK_ARTIFACT_FORMAT_VERSION: u32 = 1;
pub(super) const AES_GCM_NONCE_LEN: usize = 12;

fn sha256_digest(bytes: &[u8]) -> super::pack::BackupDigest {
    use sha2::{Digest, Sha256};

    super::pack::BackupDigest::sha256(hex::encode(Sha256::digest(bytes)))
}
