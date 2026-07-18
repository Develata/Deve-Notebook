//! plan_ref:
//!   - 03_storage/index#remote-import-runtime-layout
//!   - 06_backup#projection-backup-secret-ref-contract

use super::{
    BLOBS_DIR, CANDIDATES_DIR, MANIFEST_FILE, RemoteImportArtifactRoot, candidate_file,
    capture::{verify_blob, write_new_synced},
    durability::{publish_file_no_replace, sync_parent},
};
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::manifest::{
    EncodedCandidate, ManifestEntry, decode_manifest, digest_entry_set, verify_candidate,
};
use crate::remote_import::types::RemoteImportSessionRecord;
use std::collections::BTreeSet;
use std::io::Read;

const MAX_JSON_ARTIFACT_BYTES: u64 = 8 * 1024 * 1024;

pub(in crate::remote_import) fn verify_published_session(
    root: &RemoteImportArtifactRoot,
    record: &RemoteImportSessionRecord,
) -> RemoteImportResult<Vec<ManifestEntry>> {
    let source = record.source_snapshot.as_ref().ok_or_else(|| {
        RemoteImportError::ArtifactTampered("session source snapshot is missing".to_string())
    })?;
    let candidate = record.candidate.as_ref().ok_or_else(|| {
        RemoteImportError::ArtifactTampered("session candidate is missing".to_string())
    })?;
    let session = root.checked_session_path(record.session_id)?;
    let manifest_bytes = read_bounded_json(&session.join(MANIFEST_FILE))?;
    if crate::remote_import::types::RemoteImportDigest::of(&manifest_bytes)
        != source.manifest_digest
    {
        return Err(RemoteImportError::ArtifactTampered(
            "source manifest digest mismatch".to_string(),
        ));
    }
    let entries = decode_manifest(&manifest_bytes)?;
    let payload_bytes = entries.iter().try_fold(0u64, |total, entry| {
        total.checked_add(entry.size).ok_or_else(|| {
            RemoteImportError::ArtifactTampered("source payload size overflow".to_string())
        })
    })?;
    if entries.len() as u32 != source.file_count
        || payload_bytes != source.payload_bytes
        || digest_entry_set(&entries) != source.blob_set_digest
    {
        return Err(RemoteImportError::ArtifactTampered(
            "source snapshot aggregate mismatch".to_string(),
        ));
    }
    for entry in &entries {
        verify_blob(
            &session.join(BLOBS_DIR).join(entry.digest.to_hex()),
            entry.digest,
            entry.size,
        )?;
    }
    let candidate_bytes = read_bounded_json(
        &session
            .join(CANDIDATES_DIR)
            .join(candidate_file(candidate.revision.get())),
    )?;
    verify_candidate(&candidate_bytes, candidate)?;
    root.verify()?;
    Ok(entries)
}

pub(in crate::remote_import) fn verify_exact_published_session(
    root: &RemoteImportArtifactRoot,
    record: &RemoteImportSessionRecord,
) -> RemoteImportResult<()> {
    let manifest = verify_published_session(root, record)?;
    let inventory = root.inventory_session_layout(record.session_id)?;
    if !inventory.unknown_entries.is_empty() {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "session contains unknown artifacts: {:?}",
            inventory.unknown_entries
        )));
    }
    let expected = manifest
        .into_iter()
        .map(|entry| entry.digest.to_hex())
        .collect::<BTreeSet<_>>();
    let actual = inventory.blob_names.into_iter().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(RemoteImportError::ArtifactTampered(
            "session blob inventory does not exactly match manifest".to_string(),
        ));
    }
    Ok(())
}

pub(in crate::remote_import) fn publish_candidate_revision(
    root: &RemoteImportArtifactRoot,
    record: &RemoteImportSessionRecord,
    candidate: &EncodedCandidate,
) -> RemoteImportResult<()> {
    let session = root.checked_session_path(record.session_id)?;
    let candidates = session.join(CANDIDATES_DIR);
    let final_path = candidates.join(candidate_file(candidate.record.revision.get()));
    let temp = candidates.join(format!(
        ".{}.preparing-{}",
        candidate.record.revision.get(),
        uuid::Uuid::new_v4()
    ));
    write_new_synced(&temp, &candidate.bytes)?;
    root.verify()?;
    match publish_file_no_replace(&temp, &final_path) {
        Ok(()) => {}
        Err(RemoteImportError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = read_bounded_json(&final_path);
            let matches = existing
                .as_deref()
                .is_ok_and(|bytes| verify_candidate(bytes, &candidate.record).is_ok());
            std::fs::remove_file(&temp)?;
            sync_parent(&temp)?;
            if !matches {
                return Err(RemoteImportError::CandidateRevisionConflict {
                    revision: candidate.record.revision,
                });
            }
        }
        Err(error) => {
            let _ = std::fs::remove_file(&temp);
            return Err(error);
        }
    }
    root.verify()?;
    verify_candidate(&read_bounded_json(&final_path)?, &candidate.record)
}

fn read_bounded_json(path: &std::path::Path) -> RemoteImportResult<Vec<u8>> {
    let before = std::fs::symlink_metadata(path)?;
    if !before.is_file()
        || before.file_type().is_symlink()
        || before.len() > MAX_JSON_ARTIFACT_BYTES
    {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "invalid JSON artifact metadata at {:?}",
            path
        )));
    }
    let before_identity = file_id::get_file_id(path).map_err(|error| {
        RemoteImportError::ArtifactTampered(format!(
            "failed to fingerprint JSON artifact {:?}: {error}",
            path
        ))
    })?;
    let file = std::fs::File::open(path)?;
    let opened = file.metadata()?;
    if !opened.is_file() || opened.len() != before.len() {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "JSON artifact changed while opening {:?}",
            path
        )));
    }
    let mut bytes = Vec::with_capacity(opened.len() as usize);
    file.take(MAX_JSON_ARTIFACT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_JSON_ARTIFACT_BYTES {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "JSON artifact exceeds size limit at {:?}",
            path
        )));
    }
    let after = std::fs::symlink_metadata(path)?;
    let after_identity = file_id::get_file_id(path).map_err(|error| {
        RemoteImportError::ArtifactTampered(format!(
            "failed to refingerprint JSON artifact {:?}: {error}",
            path
        ))
    })?;
    if !after.is_file()
        || after.file_type().is_symlink()
        || after.len() != opened.len()
        || after_identity != before_identity
    {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "JSON artifact changed while reading {:?}",
            path
        )));
    }
    Ok(bytes)
}
