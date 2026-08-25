//! plan_ref:
//!   - 03_storage/index#remote-import-runtime-layout
//!   - 03_storage/repair#remote-import-cleanup-repair
//!   - 06_backup#projection-backup-secret-ref-contract

mod cleanup;
pub(in crate::remote_import) use cleanup::{
    RemoteImportArtifactRemovalCheckpoint, RemoteImportArtifactRemovalPlan,
};

use super::durability::{publish_directory_no_replace, sync_directory_checked};
use super::{BLOBS_DIR, CANDIDATES_DIR};
use crate::models::RepoId;
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::types::RemoteImportSessionId;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::remote_import) enum ArtifactEntry {
    Session(RemoteImportSessionId),
    Preparing(String),
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::remote_import) enum CandidateArtifactEntry {
    Revision(u64),
    Preparing(String),
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::remote_import) struct SessionArtifactInventory {
    pub(in crate::remote_import) blob_names: Vec<String>,
    pub(in crate::remote_import) unknown_entries: Vec<String>,
}

const MAX_ARTIFACT_TREE_NODES: usize = 65_536;
const MAX_ARTIFACT_TREE_DEPTH: usize = 8;

#[derive(Clone)]
pub(in crate::remote_import) struct RemoteImportArtifactRoot {
    path: PathBuf,
    canonical: PathBuf,
    identity: file_id::FileId,
}

impl RemoteImportArtifactRoot {
    pub(in crate::remote_import) fn open(
        ledger_root: &Path,
        repo_id: RepoId,
    ) -> RemoteImportResult<Self> {
        validate_existing_directory(ledger_root)?;
        let host = crate::utils::notegit::host_dir(ledger_root);
        create_checked_directory(&host)?;
        let imports = host.join("remote-imports");
        create_checked_directory(&imports)?;
        let path = imports.join(repo_id.to_string());
        create_checked_directory(&path)?;
        Self::pin_existing(ledger_root, path)
    }

    pub(in crate::remote_import) fn open_existing(
        ledger_root: &Path,
        repo_id: RepoId,
    ) -> RemoteImportResult<Option<Self>> {
        validate_existing_directory(ledger_root)?;
        let host = crate::utils::notegit::host_dir(ledger_root);
        if !validate_optional_directory(&host)? {
            return Ok(None);
        }
        let imports = host.join("remote-imports");
        if !validate_optional_directory(&imports)? {
            return Ok(None);
        }
        let path = imports.join(repo_id.to_string());
        if !validate_optional_directory(&path)? {
            return Ok(None);
        }
        Self::pin_existing(ledger_root, path).map(Some)
    }

    fn pin_existing(ledger_root: &Path, path: PathBuf) -> RemoteImportResult<Self> {
        validate_existing_directory(&path)?;
        let ledger_canonical = std::fs::canonicalize(ledger_root)?;
        let canonical = std::fs::canonicalize(&path)?;
        if !canonical.starts_with(&ledger_canonical) {
            return Err(RemoteImportError::UnsafeArtifactRoot(format!(
                "canonical artifact root {:?} escapes ledger root {:?}",
                canonical, ledger_canonical
            )));
        }
        let identity = file_id::get_file_id(&path).map_err(|error| {
            RemoteImportError::UnsafeArtifactRoot(format!(
                "failed to pin artifact root identity for {:?}: {}",
                path, error
            ))
        })?;
        Ok(Self {
            path,
            canonical,
            identity,
        })
    }

    pub(super) fn verify(&self) -> RemoteImportResult<()> {
        validate_existing_directory(&self.path)?;
        let canonical = std::fs::canonicalize(&self.path)?;
        let identity = file_id::get_file_id(&self.path).map_err(|error| {
            RemoteImportError::UnsafeArtifactRoot(format!(
                "failed to recheck artifact root identity for {:?}: {}",
                self.path, error
            ))
        })?;
        if canonical != self.canonical || identity != self.identity {
            return Err(RemoteImportError::UnsafeArtifactRoot(format!(
                "artifact root identity changed at {:?}",
                self.path
            )));
        }
        Ok(())
    }

    pub(super) fn create_staging(
        &self,
        session_id: RemoteImportSessionId,
        generation: u64,
    ) -> RemoteImportResult<PathBuf> {
        self.verify()?;
        let name = format!(".{session_id}.preparing-{generation}");
        let staging = self.path.join(name);
        std::fs::create_dir(&staging)?;
        validate_existing_directory(&staging)?;
        std::fs::create_dir(staging.join(BLOBS_DIR))?;
        std::fs::create_dir(staging.join(CANDIDATES_DIR))?;
        self.verify()?;
        Ok(staging)
    }

    pub(super) fn publish(
        &self,
        staging: &Path,
        session_id: RemoteImportSessionId,
    ) -> RemoteImportResult<PathBuf> {
        self.verify()?;
        require_direct_child(&self.path, staging)?;
        validate_tree(staging)?;
        let final_path = self.session_path(session_id);
        match std::fs::symlink_metadata(&final_path) {
            Ok(_) => {
                return Err(RemoteImportError::UnsafeArtifactRoot(format!(
                    "session artifact destination already exists: {:?}",
                    final_path
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        self.verify()?;
        sync_directory_checked(&staging.join(BLOBS_DIR))?;
        sync_directory_checked(&staging.join(CANDIDATES_DIR))?;
        sync_directory_checked(staging)?;
        publish_directory_no_replace(staging, &final_path)?;
        sync_directory_checked(&self.path)?;
        self.verify()?;
        validate_tree(&final_path)?;
        Ok(final_path)
    }

    pub(super) fn session_path(&self, session_id: RemoteImportSessionId) -> PathBuf {
        self.path.join(session_id.to_string())
    }

    pub(super) fn checked_session_path(
        &self,
        session_id: RemoteImportSessionId,
    ) -> RemoteImportResult<PathBuf> {
        self.verify()?;
        let path = self.session_path(session_id);
        require_direct_child(&self.path, &path)?;
        validate_tree(&path)?;
        self.verify()?;
        Ok(path)
    }

    pub(in crate::remote_import) fn list_entries(&self) -> RemoteImportResult<Vec<ArtifactEntry>> {
        self.verify()?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(metadata) = metadata_if_present(&entry.path())? else {
                continue;
            };
            if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
                entries.push(ArtifactEntry::Unknown(name));
                continue;
            }
            if let Ok(id) = uuid::Uuid::parse_str(&name) {
                entries.push(ArtifactEntry::Session(
                    RemoteImportSessionId::from_uuid_for_artifact(id),
                ));
            } else if name.starts_with('.') && name.contains(".preparing-") {
                entries.push(ArtifactEntry::Preparing(name));
            } else {
                entries.push(ArtifactEntry::Unknown(name));
            }
        }
        entries.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
        self.verify()?;
        Ok(entries)
    }

    pub(in crate::remote_import) fn list_candidate_entries(
        &self,
        session_id: RemoteImportSessionId,
    ) -> RemoteImportResult<Vec<CandidateArtifactEntry>> {
        let session = self.checked_session_path(session_id)?;
        let candidates = session.join(CANDIDATES_DIR);
        validate_existing_directory(&candidates)?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&candidates)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(metadata) = metadata_if_present(&entry.path())? else {
                continue;
            };
            if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
                entries.push(CandidateArtifactEntry::Unknown(name));
                continue;
            }
            let revision = name
                .strip_suffix(".json")
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|revision| *revision > 0 && name == format!("{revision}.json"));
            if let Some(revision) = revision {
                entries.push(CandidateArtifactEntry::Revision(revision));
            } else if name.starts_with('.') && name.contains(".preparing-") {
                entries.push(CandidateArtifactEntry::Preparing(name));
            } else {
                entries.push(CandidateArtifactEntry::Unknown(name));
            }
        }
        entries.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
        self.verify()?;
        Ok(entries)
    }

    pub(in crate::remote_import) fn inventory_session_layout(
        &self,
        session_id: RemoteImportSessionId,
    ) -> RemoteImportResult<SessionArtifactInventory> {
        let session = self.checked_session_path(session_id)?;
        let mut blob_names = Vec::new();
        let mut unknown_entries = Vec::new();
        for entry in std::fs::read_dir(&session)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(metadata) = metadata_if_present(&entry.path())? else {
                continue;
            };
            let ordinary_file =
                metadata.is_file() && !metadata.file_type().is_symlink() && !is_reparse(&metadata);
            let ordinary_dir =
                metadata.is_dir() && !metadata.file_type().is_symlink() && !is_reparse(&metadata);
            match name.as_str() {
                super::MANIFEST_FILE if ordinary_file => {}
                BLOBS_DIR if ordinary_dir => {
                    for blob in std::fs::read_dir(entry.path())? {
                        let blob = blob?;
                        let blob_name = blob.file_name().to_string_lossy().into_owned();
                        let Some(blob_metadata) = metadata_if_present(&blob.path())? else {
                            continue;
                        };
                        if blob_metadata.is_file()
                            && !blob_metadata.file_type().is_symlink()
                            && !is_reparse(&blob_metadata)
                            && is_canonical_digest_name(&blob_name)
                        {
                            blob_names.push(blob_name);
                        } else {
                            unknown_entries.push(format!("{BLOBS_DIR}/{blob_name}"));
                        }
                    }
                }
                CANDIDATES_DIR if ordinary_dir => {}
                _ => unknown_entries.push(name),
            }
        }
        blob_names.sort();
        unknown_entries.sort();
        self.verify()?;
        Ok(SessionArtifactInventory {
            blob_names,
            unknown_entries,
        })
    }
}

fn create_checked_directory(path: &Path) -> RemoteImportResult<()> {
    match std::fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    validate_existing_directory(path)
}

fn validate_existing_directory(path: &Path) -> RemoteImportResult<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata)
    {
        return Err(RemoteImportError::UnsafeArtifactRoot(format!(
            "expected ordinary directory at {:?}",
            path
        )));
    }
    Ok(())
}

fn validate_optional_directory(path: &Path) -> RemoteImportResult<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            validate_existing_directory(path)?;
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn metadata_if_present(path: &Path) -> RemoteImportResult<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn validate_tree(path: &Path) -> RemoteImportResult<()> {
    let mut visited = 0usize;
    validate_tree_inner(path, 0, &mut visited)
}

fn validate_tree_inner(path: &Path, depth: usize, visited: &mut usize) -> RemoteImportResult<()> {
    admit_tree_node(path, depth, visited)?;
    let metadata = std::fs::symlink_metadata(path)?;
    validate_tree_metadata(path, metadata, depth, visited)
}

fn validate_tree_child(path: &Path, depth: usize, visited: &mut usize) -> RemoteImportResult<()> {
    admit_tree_node(path, depth, visited)?;
    // Directory enumeration is a point-in-time observation. A child may be
    // atomically published or cleaned before its metadata is read; an absent
    // object cannot introduce a symlink/reparse escape. Exact authority files
    // are still opened and identity-checked by their dedicated readers.
    let Some(metadata) = metadata_if_present(path)? else {
        return Ok(());
    };
    validate_tree_metadata(path, metadata, depth, visited)
}

fn admit_tree_node(path: &Path, depth: usize, visited: &mut usize) -> RemoteImportResult<()> {
    *visited = visited.saturating_add(1);
    if *visited > MAX_ARTIFACT_TREE_NODES || depth > MAX_ARTIFACT_TREE_DEPTH {
        return Err(RemoteImportError::UnsafeArtifactRoot(format!(
            "artifact tree exceeds inventory safety bound at {:?}",
            path
        )));
    }
    Ok(())
}

fn validate_tree_metadata(
    path: &Path,
    metadata: std::fs::Metadata,
    depth: usize,
    visited: &mut usize,
) -> RemoteImportResult<()> {
    if metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(RemoteImportError::UnsafeArtifactRoot(format!(
            "symlink/reparse object is forbidden in artifact tree: {:?}",
            path
        )));
    }
    if metadata.is_dir() {
        for entry in std::fs::read_dir(path)? {
            validate_tree_child(&entry?.path(), depth + 1, visited)?;
        }
    } else if !metadata.is_file() {
        return Err(RemoteImportError::UnsafeArtifactRoot(format!(
            "non-file artifact object is forbidden: {:?}",
            path
        )));
    }
    Ok(())
}

fn is_canonical_digest_name(name: &str) -> bool {
    name.len() == 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_direct_child(root: &Path, child: &Path) -> RemoteImportResult<()> {
    if child.parent() != Some(root) {
        return Err(RemoteImportError::UnsafeArtifactRoot(format!(
            "artifact target {:?} is not a direct child of {:?}",
            child, root
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(_metadata: &std::fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_validation_tolerates_child_removed_after_directory_enumeration() {
        let directory = tempfile::tempdir().expect("artifact directory");
        let child = directory.path().join(".2.preparing-test");
        std::fs::write(&child, b"candidate").expect("temporary candidate");
        std::fs::remove_file(&child).expect("publish temporary candidate");
        let mut visited = 0;

        validate_tree_child(&child, 1, &mut visited)
            .expect("a vanished enumerated child is absent, not an unsafe artifact");
        let error = validate_tree(&child).expect_err("the exact tree root must still exist");
        assert!(
            matches!(error, RemoteImportError::Io(ref error) if error.kind() == std::io::ErrorKind::NotFound)
        );
    }
}
