//! plan_ref:
//!   - 06_backup#backup-branch-binding-contract
//!   - 06_backup#backup-remote-layout-contract
//!   - 06_backup#backup-pack-contract
//!
//! Branch manifest metadata and pack reference validation.

use super::{
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BackupBranchManifest, BackupBranchManifestInput,
    BackupBranchManifestPackRef,
};
use crate::backup::locator::{normalize_remote_path, safe_writer_identity};
use crate::backup::pack::pack_file_name;
use crate::backup::{BackupPackError, BackupPackPlanInput, plan_backup_pack};
use crate::models::RepoId;
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupBranchManifestError {
    #[error("backup branch manifest format version is unsupported: {0}")]
    UnsupportedFormatVersion(u32),
    #[error("backup branch manifest repo id does not match expected repo")]
    RepoIdMismatch,
    #[error("backup branch manifest writer identity does not match locator writer")]
    WriterIdentityMismatch,
    #[error("backup branch manifest path does not match locator branch path")]
    BranchPathMismatch,
    #[error("backup branch manifest must contain at least one pack")]
    EmptyPackList,
    #[error("backup branch manifest pack sequence must be greater than zero")]
    InvalidPackSequence,
    #[error("backup branch manifest pack sequence is duplicated")]
    DuplicatePackSequence,
    #[error("backup branch manifest pack object path is duplicated")]
    DuplicatePackObjectPath,
    #[error("backup branch manifest pack object path is outside branch pack prefix")]
    PackPathOutsideBranchPrefix,
    #[error("backup branch manifest pack object path does not match pack sequence")]
    PackObjectPathMismatch,
    #[error("backup branch manifest digest must be sha256 hex")]
    InvalidDigest,
    #[error(transparent)]
    Pack(#[from] BackupPackError),
    #[error(transparent)]
    Locator(#[from] super::super::BackupLocatorError),
}

pub fn validate_backup_branch_manifest(
    input: BackupBranchManifestInput,
) -> Result<BackupBranchManifest, BackupBranchManifestError> {
    if input.format_version != BACKUP_BRANCH_MANIFEST_FORMAT_VERSION {
        return Err(BackupBranchManifestError::UnsupportedFormatVersion(
            input.format_version,
        ));
    }
    if input.manifest_repo_id != input.expected_repo_id {
        return Err(BackupBranchManifestError::RepoIdMismatch);
    }

    let writer_identity = safe_writer_identity(&input.manifest_writer_identity)?;
    if writer_identity != input.branch.writer_identity {
        return Err(BackupBranchManifestError::WriterIdentityMismatch);
    }

    let branch_path = normalize_remote_path(&input.manifest_branch_path)?;
    if branch_path != input.branch.branch_path {
        return Err(BackupBranchManifestError::BranchPathMismatch);
    }

    let pack_prefix = input.branch.pack_prefix();
    let packs = validate_pack_refs(
        &pack_prefix,
        input.expected_repo_id,
        &writer_identity,
        &branch_path,
        input.packs,
    )?;

    Ok(BackupBranchManifest {
        repo_id: input.expected_repo_id,
        writer_identity,
        branch_path,
        branch_manifest_path: input.branch.branch_manifest_path(),
        pack_prefix,
        format_version: BACKUP_BRANCH_MANIFEST_FORMAT_VERSION,
        packs,
    })
}

fn validate_pack_refs(
    pack_prefix: &str,
    repo_id: RepoId,
    writer_identity: &str,
    branch_path: &str,
    packs: Vec<BackupBranchManifestPackRef>,
) -> Result<Vec<BackupBranchManifestPackRef>, BackupBranchManifestError> {
    if packs.is_empty() {
        return Err(BackupBranchManifestError::EmptyPackList);
    }

    let mut sequences = HashSet::with_capacity(packs.len());
    let mut object_paths = HashSet::with_capacity(packs.len());
    let mut normalized = Vec::with_capacity(packs.len());
    for pack in packs {
        let pack = validate_pack_ref(
            pack,
            pack_prefix,
            repo_id,
            writer_identity,
            branch_path,
            &mut sequences,
            &mut object_paths,
        )?;
        normalized.push(pack);
    }
    Ok(normalized)
}

fn validate_pack_ref(
    pack: BackupBranchManifestPackRef,
    pack_prefix: &str,
    repo_id: RepoId,
    writer_identity: &str,
    branch_path: &str,
    sequences: &mut HashSet<u64>,
    object_paths: &mut HashSet<String>,
) -> Result<BackupBranchManifestPackRef, BackupBranchManifestError> {
    if pack.pack_sequence == 0 {
        return Err(BackupBranchManifestError::InvalidPackSequence);
    }
    if !sequences.insert(pack.pack_sequence) {
        return Err(BackupBranchManifestError::DuplicatePackSequence);
    }
    let object_path = normalize_remote_path(&pack.object_path)?;
    if object_path == pack_prefix || !object_path.starts_with(&format!("{pack_prefix}/")) {
        return Err(BackupBranchManifestError::PackPathOutsideBranchPrefix);
    }
    if !object_paths.insert(object_path.clone()) {
        return Err(BackupBranchManifestError::DuplicatePackObjectPath);
    }
    if object_path != format!("{pack_prefix}/{}", pack_file_name(pack.pack_sequence)) {
        return Err(BackupBranchManifestError::PackObjectPathMismatch);
    }

    if !pack.payload_digest.is_valid_sha256() {
        return Err(BackupBranchManifestError::InvalidDigest);
    }
    let manifest = plan_backup_pack(BackupPackPlanInput {
        repo_id,
        writer_identity: writer_identity.to_owned(),
        branch_path: branch_path.to_owned(),
        pack_sequence: pack.pack_sequence,
        ledger_seq_range: pack.ledger_seq_range,
        ledger_event_count: pack.ledger_event_count,
        snapshot_count: pack.snapshot_count,
        payload_digest: pack.payload_digest,
        blob_refs: pack.blob_refs,
    })?;
    if object_path != manifest.pack_object_path() {
        return Err(BackupBranchManifestError::PackObjectPathMismatch);
    }

    Ok(BackupBranchManifestPackRef {
        pack_sequence: manifest.pack_sequence,
        object_path,
        payload_digest: manifest.payload_digest,
        ledger_seq_range: manifest.ledger_seq_range,
        ledger_event_count: manifest.ledger_event_count,
        snapshot_count: manifest.snapshot_count,
        blob_refs: manifest.blob_refs,
    })
}
