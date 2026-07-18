//! plan_ref:
//!   - 03_storage/index#remote-import-runtime-layout
//!   - 06_backup#remote-import-session-contract
//!   - 06_backup#remote-import-resource-contract

use super::error::{RemoteImportError, RemoteImportResult};
use super::types::{
    RemoteImportBaseline, RemoteImportCandidateEntry, RemoteImportCandidateRevision,
    RemoteImportCandidateRevisionRecord, RemoteImportChangeKind, RemoteImportDigest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MANIFEST_SCHEMA: &str = "deve.remote-import.source-manifest";
const CANDIDATE_SCHEMA: &str = "deve.remote-import.candidate";
const JSON_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManifestEntry {
    pub(super) path: String,
    pub(super) digest: RemoteImportDigest,
    pub(super) size: u64,
}

#[derive(Debug, Clone)]
pub(super) struct EncodedManifest {
    pub(super) bytes: Vec<u8>,
    pub(super) digest: RemoteImportDigest,
    pub(super) blob_set_digest: RemoteImportDigest,
    pub(super) entries: Vec<ManifestEntry>,
    pub(super) payload_bytes: u64,
}

#[derive(Debug, Clone)]
pub(super) struct EncodedCandidate {
    pub(super) bytes: Vec<u8>,
    pub(super) record: RemoteImportCandidateRevisionRecord,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceManifestJson {
    schema: String,
    version: u16,
    file_count: u32,
    payload_bytes: u64,
    entries: Vec<ManifestEntryJson>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestEntryJson {
    path: String,
    size: u64,
    sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateJson {
    schema: String,
    version: u16,
    revision: u64,
    ledger_head: crate::models::GlobalSeq,
    ignore_sha256: String,
    locator_sha256: String,
    entry_count: u32,
    entries: Vec<CandidateEntryJson>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateEntryJson {
    entry_id: String,
    path: String,
    blob_sha256: String,
    size: u64,
    change_kind: RemoteImportChangeKind,
    blockers: Vec<super::types::RemoteImportBlocker>,
}

pub(super) fn encode_manifest(
    mut entries: Vec<ManifestEntry>,
) -> RemoteImportResult<EncodedManifest> {
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    let payload_bytes = entries.iter().try_fold(0u64, |total, entry| {
        total
            .checked_add(entry.size)
            .ok_or(RemoteImportError::LimitExceeded {
                kind: "total payload",
                limit: super::artifact::MAX_TOTAL_PAYLOAD_BYTES,
                observed: u64::MAX,
            })
    })?;
    let file_count =
        u32::try_from(entries.len()).map_err(|_| RemoteImportError::LimitExceeded {
            kind: "file count",
            limit: super::artifact::MAX_FILE_COUNT as u64,
            observed: entries.len() as u64,
        })?;
    let json = SourceManifestJson {
        schema: MANIFEST_SCHEMA.to_string(),
        version: JSON_VERSION,
        file_count,
        payload_bytes,
        entries: entries
            .iter()
            .map(|entry| ManifestEntryJson {
                path: entry.path.clone(),
                size: entry.size,
                sha256: entry.digest.to_hex(),
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&json).map_err(RemoteImportError::json)?;
    let digest = RemoteImportDigest::of(&bytes);
    let blob_set_digest = digest_entry_set(&entries);
    Ok(EncodedManifest {
        bytes,
        digest,
        blob_set_digest,
        entries,
        payload_bytes,
    })
}

pub(super) fn decode_manifest(bytes: &[u8]) -> RemoteImportResult<Vec<ManifestEntry>> {
    let manifest: SourceManifestJson =
        serde_json::from_slice(bytes).map_err(RemoteImportError::json)?;
    if manifest.schema != MANIFEST_SCHEMA || manifest.version != JSON_VERSION {
        return Err(RemoteImportError::ArtifactTampered(
            "unsupported source manifest schema/version".to_string(),
        ));
    }
    if manifest.file_count as usize != manifest.entries.len() {
        return Err(RemoteImportError::ArtifactTampered(
            "source manifest file_count mismatch".to_string(),
        ));
    }
    if manifest.entries.len() > super::artifact::MAX_FILE_COUNT {
        return Err(RemoteImportError::ArtifactTampered(
            "source manifest exceeds file-count budget".to_string(),
        ));
    }
    let mut path_bytes = 0usize;
    let mut entries = manifest
        .entries
        .into_iter()
        .map(|entry| {
            super::artifact::validate_remote_path(&entry.path).map_err(|error| {
                RemoteImportError::ArtifactTampered(format!(
                    "source manifest contains invalid path: {error}"
                ))
            })?;
            path_bytes = path_bytes.checked_add(entry.path.len()).ok_or_else(|| {
                RemoteImportError::ArtifactTampered(
                    "source manifest path-byte total overflow".to_string(),
                )
            })?;
            if path_bytes > super::artifact::MAX_TOTAL_PATH_BYTES
                || entry.size > super::artifact::MAX_FILE_PAYLOAD_BYTES
            {
                return Err(RemoteImportError::ArtifactTampered(
                    "source manifest exceeds path or file-size budget".to_string(),
                ));
            }
            Ok(ManifestEntry {
                path: entry.path,
                digest: parse_digest(&entry.sha256)?,
                size: entry.size,
            })
        })
        .collect::<RemoteImportResult<Vec<_>>>()?;
    let original_paths = entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<Vec<_>>();
    if !original_paths.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(RemoteImportError::ArtifactTampered(
            "source manifest paths are not strictly sorted".to_string(),
        ));
    }
    let payload_bytes = entries.iter().try_fold(0u64, |total, entry| {
        total.checked_add(entry.size).ok_or_else(|| {
            RemoteImportError::ArtifactTampered("source manifest size overflow".to_string())
        })
    })?;
    if payload_bytes != manifest.payload_bytes {
        return Err(RemoteImportError::ArtifactTampered(
            "source manifest payload_bytes mismatch".to_string(),
        ));
    }
    if payload_bytes > super::artifact::MAX_TOTAL_PAYLOAD_BYTES {
        return Err(RemoteImportError::ArtifactTampered(
            "source manifest exceeds total payload budget".to_string(),
        ));
    }
    entries.shrink_to_fit();
    Ok(entries)
}

pub(super) fn encode_candidate(
    manifest: &[ManifestEntry],
    baseline: &RemoteImportBaseline,
    revision: RemoteImportCandidateRevision,
) -> RemoteImportResult<EncodedCandidate> {
    let entries = manifest
        .iter()
        .map(|source| {
            let change_kind = match baseline.existing.get(&source.path) {
                None => RemoteImportChangeKind::Added,
                Some(current) if current == &source.digest => RemoteImportChangeKind::Unchanged,
                Some(_) => RemoteImportChangeKind::Modified,
            };
            RemoteImportCandidateEntry {
                entry_id: entry_id(&source.path, source.digest),
                path: source.path.clone(),
                blob_digest: source.digest,
                size: source.size,
                change_kind,
                blockers: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    let entry_count =
        u32::try_from(entries.len()).map_err(|_| RemoteImportError::LimitExceeded {
            kind: "candidate entry count",
            limit: super::artifact::MAX_FILE_COUNT as u64,
            observed: entries.len() as u64,
        })?;
    let json = CandidateJson {
        schema: CANDIDATE_SCHEMA.to_string(),
        version: JSON_VERSION,
        revision: revision.get(),
        ledger_head: baseline.ledger_head,
        ignore_sha256: baseline.ignore_digest.to_hex(),
        locator_sha256: baseline.locator_digest.to_hex(),
        entry_count,
        entries: entries
            .iter()
            .map(|entry| CandidateEntryJson {
                entry_id: entry.entry_id.to_hex(),
                path: entry.path.clone(),
                blob_sha256: entry.blob_digest.to_hex(),
                size: entry.size,
                change_kind: entry.change_kind,
                blockers: entry.blockers.clone(),
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&json).map_err(RemoteImportError::json)?;
    let candidate_digest = RemoteImportDigest::of(&bytes);
    Ok(EncodedCandidate {
        bytes,
        record: RemoteImportCandidateRevisionRecord {
            revision,
            candidate_digest,
            ledger_head: baseline.ledger_head,
            ignore_digest: baseline.ignore_digest,
            locator_digest: baseline.locator_digest,
            entry_count,
        },
    })
}

pub(super) fn verify_candidate(
    bytes: &[u8],
    expected: &RemoteImportCandidateRevisionRecord,
) -> RemoteImportResult<()> {
    decode_candidate(bytes, expected).map(|_| ())
}

pub(super) fn decode_candidate(
    bytes: &[u8],
    expected: &RemoteImportCandidateRevisionRecord,
) -> RemoteImportResult<Vec<RemoteImportCandidateEntry>> {
    if RemoteImportDigest::of(bytes) != expected.candidate_digest {
        return Err(RemoteImportError::ArtifactTampered(
            "candidate digest mismatch".to_string(),
        ));
    }
    let candidate: CandidateJson =
        serde_json::from_slice(bytes).map_err(RemoteImportError::json)?;
    if candidate.schema != CANDIDATE_SCHEMA
        || candidate.version != JSON_VERSION
        || candidate.revision != expected.revision.get()
        || candidate.ledger_head != expected.ledger_head
        || parse_digest(&candidate.ignore_sha256)? != expected.ignore_digest
        || parse_digest(&candidate.locator_sha256)? != expected.locator_digest
        || candidate.entry_count != expected.entry_count
        || candidate.entry_count as usize != candidate.entries.len()
        || candidate.entries.len() > super::artifact::MAX_FILE_COUNT
    {
        return Err(RemoteImportError::ArtifactTampered(
            "candidate metadata mismatch".to_string(),
        ));
    }
    let paths = candidate
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<Vec<_>>();
    if !paths.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(RemoteImportError::ArtifactTampered(
            "candidate paths are not strictly sorted".to_string(),
        ));
    }
    let mut path_bytes = 0usize;
    let mut entries = Vec::with_capacity(candidate.entries.len());
    for entry in candidate.entries {
        super::artifact::validate_remote_path(&entry.path).map_err(|error| {
            RemoteImportError::ArtifactTampered(format!("candidate contains invalid path: {error}"))
        })?;
        path_bytes = path_bytes.checked_add(entry.path.len()).ok_or_else(|| {
            RemoteImportError::ArtifactTampered("candidate path-byte total overflow".to_string())
        })?;
        if path_bytes > super::artifact::MAX_TOTAL_PATH_BYTES
            || entry.size > super::artifact::MAX_FILE_PAYLOAD_BYTES
        {
            return Err(RemoteImportError::ArtifactTampered(
                "candidate exceeds path or file-size budget".to_string(),
            ));
        }
        let digest = parse_digest(&entry.blob_sha256)?;
        let parsed_entry_id = parse_digest(&entry.entry_id)?;
        if parsed_entry_id != entry_id(&entry.path, digest) {
            return Err(RemoteImportError::ArtifactTampered(
                "candidate entry_id mismatch".to_string(),
            ));
        }
        entries.push(RemoteImportCandidateEntry {
            entry_id: parsed_entry_id,
            path: entry.path,
            blob_digest: digest,
            size: entry.size,
            change_kind: entry.change_kind,
            blockers: entry.blockers,
        });
    }
    Ok(entries)
}

pub(super) fn digest_entry_set(entries: &[ManifestEntry]) -> RemoteImportDigest {
    let mut hasher = Sha256::new();
    hasher.update(b"deve-remote-import-blob-set-v1\0");
    for entry in entries {
        hasher.update((entry.path.len() as u64).to_be_bytes());
        hasher.update(entry.path.as_bytes());
        hasher.update(entry.size.to_be_bytes());
        hasher.update(entry.digest.as_bytes());
    }
    RemoteImportDigest::from_bytes(hasher.finalize().into())
}

fn entry_id(path: &str, digest: RemoteImportDigest) -> RemoteImportDigest {
    let mut hasher = Sha256::new();
    hasher.update(b"deve-remote-import-entry-v1\0");
    hasher.update((path.len() as u64).to_be_bytes());
    hasher.update(path.as_bytes());
    hasher.update(digest.as_bytes());
    RemoteImportDigest::from_bytes(hasher.finalize().into())
}

fn parse_digest(value: &str) -> RemoteImportResult<RemoteImportDigest> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RemoteImportError::ArtifactTampered(
            "digest is not canonical lowercase SHA-256 hex".to_string(),
        ));
    }
    let decoded = hex::decode(value).map_err(RemoteImportError::json)?;
    let bytes: [u8; 32] = decoded.try_into().map_err(|_| {
        RemoteImportError::ArtifactTampered("digest has invalid length".to_string())
    })?;
    Ok(RemoteImportDigest::from_bytes(bytes))
}
