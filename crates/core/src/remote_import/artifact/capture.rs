//! plan_ref:
//!   - 03_storage/index#remote-import-runtime-layout
//!   - 06_backup#remote-import-resource-contract
//!   - 06_backup#remote-import-state-machine

use super::durability::{publish_file_no_replace, sync_parent};
use super::{
    BLOBS_DIR, CANDIDATES_DIR, MANIFEST_FILE, MAX_FILE_COUNT, MAX_FILE_PAYLOAD_BYTES,
    MAX_PATH_BYTES, MAX_TOTAL_PATH_BYTES, MAX_TOTAL_PAYLOAD_BYTES, RemoteImportArtifactRoot,
    candidate_file,
};
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::manifest::{
    EncodedCandidate, ManifestEntry, encode_candidate, encode_manifest,
};
use crate::remote_import::types::{
    RemoteImportBaseline, RemoteImportCandidateRevision, RemoteImportDigest, RemoteImportSessionId,
    RemoteImportSourceSnapshot,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

pub(in crate::remote_import) struct ArtifactCapture {
    root: RemoteImportArtifactRoot,
    session_id: RemoteImportSessionId,
    staging: PathBuf,
    entries: Vec<ManifestEntry>,
    casefold_paths: BTreeSet<String>,
    payload_bytes: u64,
    path_bytes: usize,
}

pub(in crate::remote_import) struct PublishedArtifacts {
    pub(in crate::remote_import) source_snapshot: RemoteImportSourceSnapshot,
    pub(in crate::remote_import) candidate: EncodedCandidate,
    pub(in crate::remote_import) session_path: PathBuf,
    pub(in crate::remote_import) root: RemoteImportArtifactRoot,
}

impl ArtifactCapture {
    pub(in crate::remote_import) fn start(
        root: RemoteImportArtifactRoot,
        session_id: RemoteImportSessionId,
        generation: u64,
    ) -> RemoteImportResult<Self> {
        let staging = root.create_staging(session_id, generation)?;
        Ok(Self {
            root,
            session_id,
            staging,
            entries: Vec::new(),
            casefold_paths: BTreeSet::new(),
            payload_bytes: 0,
            path_bytes: 0,
        })
    }

    pub(in crate::remote_import) fn capture_file(
        &mut self,
        path: &str,
        mut reader: impl Read,
    ) -> RemoteImportResult<()> {
        validate_remote_path(path)?;
        let next_path_bytes = admit_path_budget(self.entries.len(), self.path_bytes, path.len())?;
        let casefold = path.to_lowercase();
        if !self.casefold_paths.insert(casefold) {
            return Err(RemoteImportError::DuplicatePath(path.to_string()));
        }

        let temp_path = self
            .staging
            .join(BLOBS_DIR)
            .join(format!(".capture-{}", self.entries.len()));
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        let mut hasher = Sha256::new();
        let mut file_bytes = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(RemoteImportError::source_read)?;
            if read == 0 {
                break;
            }
            let (next_file_bytes, _) =
                admit_payload_chunk(self.payload_bytes, file_bytes, read as u64)?;
            file_bytes = next_file_bytes;
            output.write_all(&buffer[..read])?;
            hasher.update(&buffer[..read]);
        }
        output.sync_all()?;
        drop(output);

        let digest = RemoteImportDigest::from_bytes(hasher.finalize().into());
        let blob_path = self.staging.join(BLOBS_DIR).join(digest.to_hex());
        match publish_file_no_replace(&temp_path, &blob_path) {
            Ok(()) => verify_blob(&blob_path, digest, file_bytes)?,
            Err(RemoteImportError::Io(error))
                if error.kind() == std::io::ErrorKind::AlreadyExists =>
            {
                verify_blob(&blob_path, digest, file_bytes)?;
                std::fs::remove_file(&temp_path)?;
                sync_parent(&temp_path)?;
            }
            Err(error) => return Err(error),
        }
        self.payload_bytes += file_bytes;
        self.path_bytes = next_path_bytes;
        self.entries.push(ManifestEntry {
            path: path.to_string(),
            digest,
            size: file_bytes,
        });
        Ok(())
    }

    pub(in crate::remote_import) fn finish(
        self,
        baseline: &RemoteImportBaseline,
    ) -> RemoteImportResult<PublishedArtifacts> {
        self.root.verify()?;
        let manifest = encode_manifest(self.entries)?;
        let candidate = encode_candidate(
            &manifest.entries,
            baseline,
            RemoteImportCandidateRevision::FIRST,
        )?;
        write_new_synced(&self.staging.join(MANIFEST_FILE), &manifest.bytes)?;
        write_new_synced(
            &self
                .staging
                .join(CANDIDATES_DIR)
                .join(candidate_file(candidate.record.revision.get())),
            &candidate.bytes,
        )?;
        let session_path = self.root.publish(&self.staging, self.session_id)?;
        Ok(PublishedArtifacts {
            source_snapshot: RemoteImportSourceSnapshot {
                manifest_digest: manifest.digest,
                blob_set_digest: manifest.blob_set_digest,
                file_count: manifest.entries.len() as u32,
                payload_bytes: manifest.payload_bytes,
            },
            candidate,
            session_path,
            root: self.root,
        })
    }
}

fn admit_path_budget(
    file_count: usize,
    total_path_bytes: usize,
    next_path_bytes: usize,
) -> RemoteImportResult<usize> {
    let next_count = file_count.saturating_add(1);
    if next_count > MAX_FILE_COUNT {
        return Err(RemoteImportError::LimitExceeded {
            kind: "file count",
            limit: MAX_FILE_COUNT as u64,
            observed: next_count as u64,
        });
    }
    let total = total_path_bytes.saturating_add(next_path_bytes);
    if total > MAX_TOTAL_PATH_BYTES {
        return Err(RemoteImportError::LimitExceeded {
            kind: "total path bytes",
            limit: MAX_TOTAL_PATH_BYTES as u64,
            observed: total as u64,
        });
    }
    Ok(total)
}

fn admit_payload_chunk(
    committed_payload_bytes: u64,
    current_file_bytes: u64,
    chunk_bytes: u64,
) -> RemoteImportResult<(u64, u64)> {
    let file_bytes =
        current_file_bytes
            .checked_add(chunk_bytes)
            .ok_or(RemoteImportError::LimitExceeded {
                kind: "file payload",
                limit: MAX_FILE_PAYLOAD_BYTES,
                observed: u64::MAX,
            })?;
    if file_bytes > MAX_FILE_PAYLOAD_BYTES {
        return Err(RemoteImportError::LimitExceeded {
            kind: "file payload",
            limit: MAX_FILE_PAYLOAD_BYTES,
            observed: file_bytes,
        });
    }
    let total = committed_payload_bytes.checked_add(file_bytes).ok_or(
        RemoteImportError::LimitExceeded {
            kind: "total payload",
            limit: MAX_TOTAL_PAYLOAD_BYTES,
            observed: u64::MAX,
        },
    )?;
    if total > MAX_TOTAL_PAYLOAD_BYTES {
        return Err(RemoteImportError::LimitExceeded {
            kind: "total payload",
            limit: MAX_TOTAL_PAYLOAD_BYTES,
            observed: total,
        });
    }
    Ok((file_bytes, total))
}

pub(in crate::remote_import) fn validate_remote_path(path: &str) -> RemoteImportResult<()> {
    if path.len() > MAX_PATH_BYTES {
        return Err(RemoteImportError::LimitExceeded {
            kind: "path bytes",
            limit: MAX_PATH_BYTES as u64,
            observed: path.len() as u64,
        });
    }
    if path.nfc().collect::<String>() != path {
        return Err(RemoteImportError::InvalidPath {
            path: path.to_string(),
            reason: "path must use NFC Unicode normalization".to_string(),
        });
    }
    crate::utils::path::validate_projection_repo_child_path(path).map_err(|error| {
        RemoteImportError::InvalidPath {
            path: path.to_string(),
            reason: error.to_string(),
        }
    })?;
    for segment in path.split('/') {
        if segment.chars().any(|character| {
            character.is_control() || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
        }) {
            return Err(RemoteImportError::InvalidPath {
                path: path.to_string(),
                reason: "path contains a host-reserved character".to_string(),
            });
        }
        let device_stem = segment.split('.').next().unwrap_or(segment);
        let upper = device_stem.to_ascii_uppercase();
        let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || upper.strip_prefix("COM").is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
            || upper.strip_prefix("LPT").is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            });
        if reserved {
            return Err(RemoteImportError::InvalidPath {
                path: path.to_string(),
                reason: "path uses a Windows reserved device name".to_string(),
            });
        }
    }
    if !(path.ends_with(".md") || path.ends_with(".markdown")) {
        return Err(RemoteImportError::InvalidPath {
            path: path.to_string(),
            reason: "only canonical lowercase Markdown extensions are accepted".to_string(),
        });
    }
    Ok(())
}

pub(super) fn verify_blob(
    path: &Path,
    expected_digest: RemoteImportDigest,
    expected_size: u64,
) -> RemoteImportResult<()> {
    read_verified_blob(path, expected_digest, expected_size).map(|_| ())
}

pub(super) fn read_verified_blob(
    path: &Path,
    expected_digest: RemoteImportDigest,
    expected_size: u64,
) -> RemoteImportResult<Vec<u8>> {
    let before = std::fs::symlink_metadata(path)?;
    if !before.is_file() || before.file_type().is_symlink() || before.len() != expected_size {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "blob metadata mismatch at {:?}",
            path
        )));
    }
    let before_identity = file_id::get_file_id(path).map_err(|error| {
        RemoteImportError::ArtifactTampered(format!(
            "failed to fingerprint blob {:?}: {error}",
            path
        ))
    })?;
    let mut file = std::fs::File::open(path)?;
    if file.metadata()?.len() != expected_size {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "blob changed while opening {:?}",
            path
        )));
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut observed_size = 0u64;
    let capacity = usize::try_from(expected_size).map_err(|_| {
        RemoteImportError::ArtifactTampered(format!("blob size does not fit memory at {:?}", path))
    })?;
    let mut bytes = Vec::with_capacity(capacity);
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        observed_size = observed_size.checked_add(read as u64).ok_or_else(|| {
            RemoteImportError::ArtifactTampered(format!("blob size overflow at {:?}", path))
        })?;
        if observed_size > expected_size {
            return Err(RemoteImportError::ArtifactTampered(format!(
                "blob grew while reading {:?}",
                path
            )));
        }
        hasher.update(&buffer[..read]);
        bytes.extend_from_slice(&buffer[..read]);
    }
    let after = std::fs::symlink_metadata(path)?;
    let after_identity = file_id::get_file_id(path).map_err(|error| {
        RemoteImportError::ArtifactTampered(format!(
            "failed to refingerprint blob {:?}: {error}",
            path
        ))
    })?;
    if observed_size != expected_size
        || !after.is_file()
        || after.file_type().is_symlink()
        || after.len() != expected_size
        || after_identity != before_identity
    {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "blob changed while reading {:?}",
            path
        )));
    }
    let actual = RemoteImportDigest::from_bytes(hasher.finalize().into());
    if actual != expected_digest {
        return Err(RemoteImportError::ArtifactTampered(format!(
            "blob digest mismatch at {:?}",
            path
        )));
    }
    Ok(bytes)
}

pub(super) fn write_new_synced(path: &Path, bytes: &[u8]) -> RemoteImportResult<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    sync_parent(path)?;
    Ok(())
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    #[test]
    fn file_count_and_path_total_share_the_declared_closed_boundary() {
        assert_eq!(
            MAX_TOTAL_PATH_BYTES,
            MAX_FILE_COUNT * MAX_PATH_BYTES,
            "total path budget is the exact implication of the two per-entry bounds"
        );
        let total = admit_path_budget(
            MAX_FILE_COUNT - 1,
            MAX_TOTAL_PATH_BYTES - MAX_PATH_BYTES,
            MAX_PATH_BYTES,
        )
        .expect("2048 paths at 1024 bytes fit exactly");
        assert_eq!(total, MAX_TOTAL_PATH_BYTES);
        assert!(matches!(
            admit_path_budget(MAX_FILE_COUNT, total, 1),
            Err(RemoteImportError::LimitExceeded {
                kind: "file count",
                ..
            })
        ));
    }

    #[test]
    fn path_and_payload_limits_reject_the_first_byte_over_budget() {
        assert!(matches!(
            validate_remote_path(&"a".repeat(MAX_PATH_BYTES + 1)),
            Err(RemoteImportError::LimitExceeded {
                kind: "path bytes",
                ..
            })
        ));
        assert!(matches!(
            admit_payload_chunk(MAX_TOTAL_PAYLOAD_BYTES, 0, 1),
            Err(RemoteImportError::LimitExceeded {
                kind: "total payload",
                ..
            })
        ));
        assert!(matches!(
            admit_payload_chunk(0, MAX_FILE_PAYLOAD_BYTES, 1),
            Err(RemoteImportError::LimitExceeded {
                kind: "file payload",
                ..
            })
        ));
    }
}
