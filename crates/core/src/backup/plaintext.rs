//! plan_ref:
//!   - 06_backup#backup-pack-plaintext-schema-contract
//!
//! Backup pack plaintext schema gate.
//!
//! This module validates decrypted backup pack plaintext bytes against the
//! already verified pack manifest. It does not append ledger entries, stage
//! source-control changes, create commit anchors, enqueue Git mirrors, or touch
//! Projection Workspaces.

use super::locator::normalize_remote_path;
use super::pack::{
    BackupBlobRef, BackupPackError, BackupPackManifest, BackupSeqRange, validate_pack_manifest,
};
use crate::codec;
use crate::models::{LedgerEntry, RepoId, deserialize_ledger_entry};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[cfg(test)]
mod tests;

pub const BACKUP_PACK_PLAINTEXT_FORMAT_VERSION: u32 = 2;
const BACKUP_PACK_PLAINTEXT_MAGIC: &[u8; 8] = b"DEVEBKP2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupPackPlaintextLedgerEntry {
    pub global_seq: u64,
    pub entry_bytes: Vec<u8>,
}

impl BackupPackPlaintextLedgerEntry {
    pub fn decode(&self) -> Result<LedgerEntry, BackupPackPlaintextError> {
        decode_ledger_entry_bytes(&self.entry_bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupPackPlaintext {
    pub format_version: u32,
    pub repo_id: RepoId,
    pub writer_identity: String,
    pub branch_path: String,
    pub pack_sequence: u64,
    pub ledger_seq_range: Option<BackupSeqRange>,
    pub ledger_entries: Vec<BackupPackPlaintextLedgerEntry>,
    pub snapshot_refs: Vec<BackupBlobRef>,
    pub blob_refs: Vec<BackupBlobRef>,
}

impl BackupPackPlaintext {
    pub fn decoded_ledger_entries(
        &self,
    ) -> Result<Vec<(u64, LedgerEntry)>, BackupPackPlaintextError> {
        self.ledger_entries
            .iter()
            .map(|entry| Ok((entry.global_seq, entry.decode()?)))
            .collect()
    }
}

pub struct BackupPackPlaintextEncodeInput<'a> {
    pub manifest: &'a BackupPackManifest,
    pub plaintext: &'a BackupPackPlaintext,
}

pub struct BackupPackPlaintextOpenInput<'a> {
    pub manifest: &'a BackupPackManifest,
    pub plaintext_bytes: &'a [u8],
}

pub struct BackupPackPlaintextValidateInput<'a> {
    pub manifest: &'a BackupPackManifest,
    pub plaintext: &'a BackupPackPlaintext,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupPackPlaintextError {
    #[error("backup pack plaintext is empty")]
    EmptyPlaintext,
    #[error("backup pack plaintext is missing DEVEBKP2 magic")]
    MissingMagic,
    #[error("backup pack plaintext format version is unsupported: {0}")]
    UnsupportedFormatVersion(u32),
    #[error("backup pack plaintext repo id does not match manifest")]
    RepoIdMismatch,
    #[error("backup pack plaintext writer identity does not match manifest")]
    WriterIdentityMismatch,
    #[error("backup pack plaintext branch path does not match manifest")]
    BranchPathMismatch,
    #[error("backup pack plaintext sequence does not match manifest")]
    PackSequenceMismatch,
    #[error("backup pack plaintext ledger range does not match manifest")]
    LedgerRangeMismatch,
    #[error("backup pack plaintext ledger entry count does not match manifest")]
    LedgerEntryCountMismatch,
    #[error("backup pack plaintext ledger sequence range is invalid")]
    InvalidLedgerRange,
    #[error("backup pack plaintext ledger entry sequence is not contiguous")]
    LedgerSequenceMismatch,
    #[error("backup pack plaintext ledger entry bytes are empty")]
    EmptyLedgerEntryBytes,
    #[error("backup pack plaintext ledger entry bytes are not a versioned ledger entry")]
    InvalidLedgerEntry,
    #[error("backup pack plaintext snapshot count does not match manifest")]
    SnapshotCountMismatch,
    #[error("backup pack plaintext blob refs do not match manifest")]
    BlobRefsMismatch,
    #[error("backup pack plaintext blob or snapshot ref is invalid")]
    InvalidBlobRef,
    #[error("backup pack plaintext serialization failed")]
    SerializeFailed,
    #[error("backup pack plaintext deserialization failed")]
    DeserializeFailed,
    #[error(transparent)]
    Pack(#[from] BackupPackError),
}

pub fn encode_backup_pack_plaintext(
    input: BackupPackPlaintextEncodeInput<'_>,
) -> Result<Vec<u8>, BackupPackPlaintextError> {
    validate_backup_pack_plaintext(BackupPackPlaintextValidateInput {
        manifest: input.manifest,
        plaintext: input.plaintext,
    })?;

    let payload =
        codec::encode(input.plaintext).map_err(|_| BackupPackPlaintextError::SerializeFailed)?;
    let mut bytes = Vec::with_capacity(BACKUP_PACK_PLAINTEXT_MAGIC.len() + payload.len());
    bytes.extend_from_slice(BACKUP_PACK_PLAINTEXT_MAGIC);
    bytes.extend(payload);
    Ok(bytes)
}

pub fn open_backup_pack_plaintext(
    input: BackupPackPlaintextOpenInput<'_>,
) -> Result<BackupPackPlaintext, BackupPackPlaintextError> {
    if input.plaintext_bytes.is_empty() {
        return Err(BackupPackPlaintextError::EmptyPlaintext);
    }
    let payload = input
        .plaintext_bytes
        .strip_prefix(BACKUP_PACK_PLAINTEXT_MAGIC)
        .ok_or(BackupPackPlaintextError::MissingMagic)?;
    let plaintext: BackupPackPlaintext =
        codec::decode(payload).map_err(|_| BackupPackPlaintextError::DeserializeFailed)?;
    validate_backup_pack_plaintext(BackupPackPlaintextValidateInput {
        manifest: input.manifest,
        plaintext: &plaintext,
    })?;
    Ok(plaintext)
}

pub fn validate_backup_pack_plaintext(
    input: BackupPackPlaintextValidateInput<'_>,
) -> Result<(), BackupPackPlaintextError> {
    let manifest = input.manifest;
    let plaintext = input.plaintext;

    validate_pack_manifest(
        manifest,
        manifest.repo_id,
        &manifest.writer_identity,
        &manifest.branch_path,
    )?;

    if plaintext.format_version != BACKUP_PACK_PLAINTEXT_FORMAT_VERSION {
        return Err(BackupPackPlaintextError::UnsupportedFormatVersion(
            plaintext.format_version,
        ));
    }
    if plaintext.repo_id != manifest.repo_id {
        return Err(BackupPackPlaintextError::RepoIdMismatch);
    }
    if plaintext.writer_identity != manifest.writer_identity {
        return Err(BackupPackPlaintextError::WriterIdentityMismatch);
    }
    if plaintext.branch_path != manifest.branch_path {
        return Err(BackupPackPlaintextError::BranchPathMismatch);
    }
    if plaintext.pack_sequence != manifest.pack_sequence {
        return Err(BackupPackPlaintextError::PackSequenceMismatch);
    }
    if plaintext.ledger_seq_range != manifest.ledger_seq_range {
        return Err(BackupPackPlaintextError::LedgerRangeMismatch);
    }
    if len_to_u64(plaintext.ledger_entries.len()) != manifest.ledger_event_count {
        return Err(BackupPackPlaintextError::LedgerEntryCountMismatch);
    }
    if len_to_u64(plaintext.snapshot_refs.len()) != manifest.snapshot_count {
        return Err(BackupPackPlaintextError::SnapshotCountMismatch);
    }
    if plaintext.blob_refs != manifest.blob_refs {
        return Err(BackupPackPlaintextError::BlobRefsMismatch);
    }

    validate_plaintext_refs(&plaintext.snapshot_refs)?;
    validate_plaintext_refs(&plaintext.blob_refs)?;
    validate_plaintext_ledger_entries(
        plaintext.ledger_seq_range,
        manifest.ledger_event_count,
        &plaintext.ledger_entries,
    )
}

fn validate_plaintext_ledger_entries(
    ledger_seq_range: Option<BackupSeqRange>,
    ledger_event_count: u64,
    entries: &[BackupPackPlaintextLedgerEntry],
) -> Result<(), BackupPackPlaintextError> {
    match (ledger_event_count, ledger_seq_range) {
        (0, None) if entries.is_empty() => return Ok(()),
        (0, _) => return Err(BackupPackPlaintextError::InvalidLedgerRange),
        (_, None) => return Err(BackupPackPlaintextError::InvalidLedgerRange),
        (_, Some(range)) => {
            let range_count = range
                .end
                .checked_sub(range.start)
                .and_then(|delta| delta.checked_add(1))
                .ok_or(BackupPackPlaintextError::InvalidLedgerRange)?;
            if range_count != ledger_event_count {
                return Err(BackupPackPlaintextError::InvalidLedgerRange);
            }

            for (idx, entry) in entries.iter().enumerate() {
                let expected_seq = range
                    .start
                    .checked_add(
                        u64::try_from(idx)
                            .map_err(|_| BackupPackPlaintextError::InvalidLedgerRange)?,
                    )
                    .ok_or(BackupPackPlaintextError::InvalidLedgerRange)?;
                if entry.global_seq != expected_seq {
                    return Err(BackupPackPlaintextError::LedgerSequenceMismatch);
                }
                decode_ledger_entry_bytes(&entry.entry_bytes)?;
            }
        }
    }
    Ok(())
}

fn decode_ledger_entry_bytes(bytes: &[u8]) -> Result<LedgerEntry, BackupPackPlaintextError> {
    if bytes.is_empty() {
        return Err(BackupPackPlaintextError::EmptyLedgerEntryBytes);
    }
    deserialize_ledger_entry(bytes).map_err(|_| BackupPackPlaintextError::InvalidLedgerEntry)
}

fn validate_plaintext_refs(refs: &[BackupBlobRef]) -> Result<(), BackupPackPlaintextError> {
    let mut seen = HashSet::with_capacity(refs.len());
    for reference in refs {
        if !reference.digest.is_valid_sha256() {
            return Err(BackupPackPlaintextError::InvalidBlobRef);
        }
        let normalized = normalize_remote_path(&reference.path)
            .map_err(|_| BackupPackPlaintextError::InvalidBlobRef)?;
        if normalized != reference.path || !seen.insert(normalized) {
            return Err(BackupPackPlaintextError::InvalidBlobRef);
        }
    }
    Ok(())
}

fn len_to_u64(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}
