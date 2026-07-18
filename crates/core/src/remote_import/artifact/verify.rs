//! plan_ref:
//!   - 03_storage/index#remote-import-runtime-layout
//!   - 06_backup#projection-backup-secret-ref-contract

use super::{
    BLOBS_DIR, CANDIDATES_DIR, MANIFEST_FILE, RemoteImportArtifactRoot, candidate_file,
    capture::{read_verified_blob, verify_blob, write_new_synced},
    durability::{publish_file_no_replace, sync_parent},
};
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::manifest::{
    EncodedCandidate, ManifestEntry, decode_candidate, decode_manifest, digest_entry_set,
    verify_candidate,
};
use crate::remote_import::types::{
    RemoteImportBlocker, RemoteImportChangeKind, RemoteImportDigest, RemoteImportSessionRecord,
};
use std::collections::BTreeSet;
use std::io::Read;

const MAX_JSON_ARTIFACT_BYTES: u64 = 8 * 1024 * 1024;

pub(in crate::remote_import) struct VerifiedRemoteImportEntry {
    pub(in crate::remote_import) entry_id: RemoteImportDigest,
    pub(in crate::remote_import) path: String,
    pub(in crate::remote_import) blob_digest: RemoteImportDigest,
    pub(in crate::remote_import) size: u64,
    pub(in crate::remote_import) change_kind: RemoteImportChangeKind,
    pub(in crate::remote_import) blockers: Vec<RemoteImportBlocker>,
    pub(in crate::remote_import) content: String,
}

struct PublishedMetadata {
    session: std::path::PathBuf,
    manifest: Vec<ManifestEntry>,
    candidate: Vec<crate::remote_import::types::RemoteImportCandidateEntry>,
}

pub(in crate::remote_import) fn verify_published_session(
    root: &RemoteImportArtifactRoot,
    record: &RemoteImportSessionRecord,
) -> RemoteImportResult<Vec<ManifestEntry>> {
    let metadata = read_published_metadata(root, record)?;
    for entry in &metadata.manifest {
        verify_blob(
            &metadata.session.join(BLOBS_DIR).join(entry.digest.to_hex()),
            entry.digest,
            entry.size,
        )?;
    }
    root.verify()?;
    Ok(metadata.manifest)
}

pub(in crate::remote_import) fn verify_apply_artifacts(
    root: &RemoteImportArtifactRoot,
    record: &RemoteImportSessionRecord,
) -> RemoteImportResult<Vec<VerifiedRemoteImportEntry>> {
    let metadata = read_published_metadata(root, record)?;
    verify_exact_inventory(root, record, &metadata.manifest)?;
    let mut verified = Vec::with_capacity(metadata.manifest.len());
    for (manifest, candidate) in metadata.manifest.iter().zip(metadata.candidate) {
        let bytes = read_verified_blob(
            &metadata
                .session
                .join(BLOBS_DIR)
                .join(manifest.digest.to_hex()),
            manifest.digest,
            manifest.size,
        )?;
        let content = String::from_utf8(bytes).map_err(|_| {
            RemoteImportError::ArtifactTampered(format!(
                "blob for {:?} is not valid UTF-8 Markdown",
                manifest.path
            ))
        })?;
        verified.push(VerifiedRemoteImportEntry {
            entry_id: candidate.entry_id,
            path: manifest.path.clone(),
            blob_digest: manifest.digest,
            size: manifest.size,
            change_kind: candidate.change_kind,
            blockers: candidate.blockers,
            content,
        });
    }
    root.verify()?;
    Ok(verified)
}

pub(in crate::remote_import) fn verify_review_artifacts(
    root: &RemoteImportArtifactRoot,
    record: &RemoteImportSessionRecord,
) -> RemoteImportResult<Vec<crate::remote_import::types::RemoteImportCandidateEntry>> {
    let metadata = read_published_metadata(root, record)?;
    verify_exact_inventory(root, record, &metadata.manifest)?;
    for entry in &metadata.manifest {
        verify_blob(
            &metadata.session.join(BLOBS_DIR).join(entry.digest.to_hex()),
            entry.digest,
            entry.size,
        )?;
    }
    root.verify()?;
    Ok(metadata.candidate)
}

fn read_published_metadata(
    root: &RemoteImportArtifactRoot,
    record: &RemoteImportSessionRecord,
) -> RemoteImportResult<PublishedMetadata> {
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
    let candidate_bytes = read_bounded_json(
        &session
            .join(CANDIDATES_DIR)
            .join(candidate_file(candidate.revision.get())),
    )?;
    let candidate_entries = decode_candidate(&candidate_bytes, candidate)?;
    if entries.len() != candidate_entries.len()
        || entries
            .iter()
            .zip(&candidate_entries)
            .any(|(manifest, candidate)| {
                manifest.path != candidate.path
                    || manifest.digest != candidate.blob_digest
                    || manifest.size != candidate.size
            })
    {
        return Err(RemoteImportError::ArtifactTampered(
            "candidate entries do not exactly match source manifest".to_string(),
        ));
    }
    Ok(PublishedMetadata {
        session,
        manifest: entries,
        candidate: candidate_entries,
    })
}

pub(in crate::remote_import) fn verify_exact_published_session(
    root: &RemoteImportArtifactRoot,
    record: &RemoteImportSessionRecord,
) -> RemoteImportResult<()> {
    let manifest = verify_published_session(root, record)?;
    verify_exact_inventory(root, record, &manifest)
}

fn verify_exact_inventory(
    root: &RemoteImportArtifactRoot,
    record: &RemoteImportSessionRecord,
    manifest: &[ManifestEntry],
) -> RemoteImportResult<()> {
    let inventory = root.inventory_session_layout(record.session_id)?;
    if !inventory.unknown_entries.is_empty() {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "session contains unknown artifacts: {:?}",
            inventory.unknown_entries
        )));
    }
    let expected = manifest
        .iter()
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
