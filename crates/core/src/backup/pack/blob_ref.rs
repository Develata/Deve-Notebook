//! plan_ref:
//!   - 06_backup#backup-pack-contract
//!
//! Backup pack blob reference validation.
//!
//! This module normalizes pack blob references only. It does not read blobs,
//! compute digests, encrypt artifacts, write providers, or touch ledger state.

use super::super::locator::normalize_remote_path;
use super::{BackupBlobRef, BackupPackError, validate_digest};
use std::collections::HashSet;

pub(super) fn normalize_blob_refs(
    blob_refs: Vec<BackupBlobRef>,
) -> Result<Vec<BackupBlobRef>, BackupPackError> {
    let mut seen = HashSet::with_capacity(blob_refs.len());
    let mut normalized = Vec::with_capacity(blob_refs.len());
    for blob in blob_refs {
        validate_digest(&blob.digest)?;
        let path = normalize_remote_path(&blob.path)?;
        if !seen.insert(path.clone()) {
            return Err(BackupPackError::DuplicateBlobPath);
        }
        normalized.push(BackupBlobRef {
            path,
            size_bytes: blob.size_bytes,
            digest: blob.digest,
        });
    }
    Ok(normalized)
}

pub(super) fn validate_blob_refs(blob_refs: &[BackupBlobRef]) -> Result<(), BackupPackError> {
    let mut seen = HashSet::with_capacity(blob_refs.len());
    for blob in blob_refs {
        validate_digest(&blob.digest)?;
        let path = normalize_remote_path(&blob.path)?;
        if !seen.insert(path) {
            return Err(BackupPackError::DuplicateBlobPath);
        }
    }
    Ok(())
}
