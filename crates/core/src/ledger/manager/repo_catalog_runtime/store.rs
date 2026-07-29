//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 03_storage/index#repo-runtime-layout

use super::RepoCatalogError;
use super::model::{CATALOG_RECORD_MAX_BYTES, RepoCatalogMembershipRecord};
use crate::models::RepoId;
use crate::utils::fs::{
    FileTryLockError, create_atomic_replace_temp, ensure_open_file_matches_path,
    open_regular_file_lock, open_regular_file_read, replace_file_atomically, sync_directory,
    try_lock_file_exclusive, unlock_file,
};
use crate::utils::notegit;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const CATALOG_DIR: &str = "repo-catalog";
const LOCK_FILE: &str = "repo-catalog.lock";
const TEMP_PREFIX: &str = ".deve-repo-catalog.";
const CATALOG_RECORD_LIMIT: usize = 65_536;

#[cfg(test)]
const POST_REPLACE_FAILURE_MARKER: &str = ".repo-catalog.inject-post-replace-sync-failure";
#[cfg(test)]
const PRE_REPLACE_FAILURE_MARKER: &str = ".repo-catalog.inject-pre-replace-failure";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RepoCatalogPublishPhase {
    BeforeReplace,
    AfterReplaceSync,
}

#[derive(Debug)]
pub(super) struct RepoCatalogPublishFailure {
    pub(super) phase: RepoCatalogPublishPhase,
    pub(super) primary: std::io::Error,
    pub(super) cleanup: Option<std::io::Error>,
}

impl std::fmt::Display for RepoCatalogPublishFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "repo catalog publish failed at {:?}: {}",
            self.phase, self.primary
        )?;
        if let Some(cleanup) = &self.cleanup {
            write!(f, "; cleanup={cleanup}")?;
        }
        Ok(())
    }
}

impl std::error::Error for RepoCatalogPublishFailure {}

pub(super) struct RepoCatalogStore {
    dir: PathBuf,
    lock_path: PathBuf,
    lock_file: File,
}

impl RepoCatalogStore {
    pub(super) fn open(ledger_dir: &Path) -> Result<Self, RepoCatalogError> {
        ensure_regular_directory(ledger_dir, "ledger root", false)?;
        let host = notegit::host_dir(ledger_dir);
        ensure_regular_directory(&host, "host runtime directory", true)?;
        let dir = host.join(CATALOG_DIR);
        ensure_regular_directory(&dir, "repo catalog directory", true)?;
        let lock_path = host.join(LOCK_FILE);
        let lock_file = open_regular_file_lock(&lock_path, "repo catalog authority lock")?;
        Ok(Self {
            dir,
            lock_path,
            lock_file,
        })
    }

    pub(super) fn lock(&self) -> Result<RepoCatalogStoreGuard<'_>, RepoCatalogError> {
        match try_lock_file_exclusive(&self.lock_file) {
            Ok(()) => {}
            Err(FileTryLockError::WouldBlock) => {
                return Err(RepoCatalogError::AuthorityBusy);
            }
            Err(FileTryLockError::Error(error)) => return Err(error.into()),
        }
        if let Err(error) = ensure_open_file_matches_path(
            &self.lock_file,
            &self.lock_path,
            "repo catalog authority lock",
        ) {
            let _ = unlock_file(&self.lock_file);
            return Err(error.into());
        }
        Ok(RepoCatalogStoreGuard {
            file: &self.lock_file,
        })
    }

    pub(super) fn load(
        &self,
        repo_id: RepoId,
    ) -> Result<Option<RepoCatalogMembershipRecord>, RepoCatalogError> {
        let path = self.record_path(repo_id);
        let file = match open_regular_file_read(&path, "repo catalog record") {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        read_record(file, &path, repo_id).map(Some)
    }

    /// Lists records while cleaning only strictly project-owned crash temps.
    /// The caller must hold the catalog cut guard so a live publisher cannot
    /// have its temp mistaken for crash debris.
    pub(super) fn list(&self) -> Result<Vec<RepoCatalogMembershipRecord>, RepoCatalogError> {
        let mut record_ids = Vec::new();
        let mut removed_temp = false;
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(|| {
                RepoCatalogError::InvalidRecord(format!(
                    "repo catalog filename is not UTF-8: {path:?}"
                ))
            })?;
            if is_owned_temp_name(name) {
                validate_regular_entry(&entry, CATALOG_RECORD_MAX_BYTES, "repo catalog temp")?;
                std::fs::remove_file(&path).map_err(|error| {
                    RepoCatalogError::InvalidRecord(format!(
                        "failed to clean repo catalog crash temp {path:?}: {error}"
                    ))
                })?;
                removed_temp = true;
                continue;
            }
            validate_regular_entry(&entry, CATALOG_RECORD_MAX_BYTES, "repo catalog record")?;
            let stem = name.strip_suffix(".json").ok_or_else(|| {
                RepoCatalogError::InvalidRecord(format!(
                    "unexpected repo catalog filename {name:?}"
                ))
            })?;
            let repo_id = Uuid::parse_str(stem).map_err(|error| {
                RepoCatalogError::InvalidRecord(format!(
                    "invalid RepoId catalog filename {name:?}: {error}"
                ))
            })?;
            if format!("{repo_id}.json") != name {
                return Err(RepoCatalogError::InvalidRecord(format!(
                    "repo catalog filename must use lowercase canonical RepoId: {name:?}"
                )));
            }
            if record_ids.len() == CATALOG_RECORD_LIMIT {
                return Err(RepoCatalogError::InvalidRecord(format!(
                    "repo catalog exceeds {CATALOG_RECORD_LIMIT} record entries"
                )));
            }
            record_ids.push(repo_id);
        }
        if removed_temp {
            sync_directory(&self.dir)?;
        }
        record_ids.sort();
        let mut records = Vec::with_capacity(record_ids.len());
        for repo_id in record_ids {
            let record =
                self.load(repo_id)?
                    .ok_or_else(|| RepoCatalogError::CutOutcomeUnknown {
                        repo_id,
                        detail: "catalog record disappeared during listing".to_string(),
                    })?;
            records.push(record);
        }
        Ok(records)
    }

    pub(super) fn publish(
        &self,
        record: &RepoCatalogMembershipRecord,
    ) -> Result<(), RepoCatalogPublishFailure> {
        if let Err(error) = record.validate(record.repo_id()) {
            return Err(before_replace_failure(
                std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()),
                None,
            ));
        }
        let bytes = canonical_record_bytes(record).map_err(|error| {
            before_replace_failure(
                std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()),
                None,
            )
        })?;
        if bytes.len() as u64 > CATALOG_RECORD_MAX_BYTES {
            return Err(before_replace_failure(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "serialized repo catalog record exceeds {CATALOG_RECORD_MAX_BYTES} bytes"
                    ),
                ),
                None,
            ));
        }
        let destination = self.record_path(record.repo_id());
        let temp = self.dir.join(format!(
            "{TEMP_PREFIX}{}.{}.{}.tmp",
            record.repo_id(),
            std::process::id(),
            Uuid::new_v4()
        ));
        let mut file = create_atomic_replace_temp(&temp)
            .map_err(|error| before_replace_failure(error, None))?;
        if let Err(primary) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
            drop(file);
            return Err(before_replace_failure(primary, Some(&temp)));
        }
        #[cfg(test)]
        if self
            .dir
            .parent()
            .is_some_and(|host| host.join(PRE_REPLACE_FAILURE_MARKER).exists())
        {
            drop(file);
            return Err(before_replace_failure(
                std::io::Error::other("injected pre-replace failure"),
                Some(&temp),
            ));
        }
        if let Err(primary) = replace_file_atomically(&file, &temp, &destination) {
            drop(file);
            return Err(before_replace_failure(primary, Some(&temp)));
        }
        drop(file);
        #[cfg(test)]
        if self
            .dir
            .parent()
            .is_some_and(|host| host.join(POST_REPLACE_FAILURE_MARKER).exists())
        {
            return Err(RepoCatalogPublishFailure {
                phase: RepoCatalogPublishPhase::AfterReplaceSync,
                primary: std::io::Error::other("injected post-replace directory sync failure"),
                cleanup: None,
            });
        }
        sync_directory(&self.dir).map_err(|primary| RepoCatalogPublishFailure {
            phase: RepoCatalogPublishPhase::AfterReplaceSync,
            primary,
            cleanup: None,
        })
    }

    pub(super) fn seal_visible_record(&self) -> Result<(), RepoCatalogError> {
        sync_directory(&self.dir).map_err(Into::into)
    }

    pub(super) fn remove_exact(
        &self,
        expected: &RepoCatalogMembershipRecord,
    ) -> Result<bool, RepoCatalogError> {
        let current = self.load(expected.repo_id())?;
        match current {
            None => Ok(false),
            Some(current) if current == *expected => {
                std::fs::remove_file(self.record_path(expected.repo_id()))?;
                sync_directory(&self.dir)?;
                Ok(true)
            }
            Some(_) => Err(RepoCatalogError::CutOutcomeUnknown {
                repo_id: expected.repo_id(),
                detail: "catalog tombstone changed before exact retirement".to_string(),
            }),
        }
    }

    #[cfg(test)]
    pub(super) fn post_replace_failure_marker(&self) -> PathBuf {
        self.dir
            .parent()
            .expect("catalog directory always has host parent")
            .join(POST_REPLACE_FAILURE_MARKER)
    }

    #[cfg(test)]
    pub(super) fn pre_replace_failure_marker(&self) -> PathBuf {
        self.dir
            .parent()
            .expect("catalog directory always has host parent")
            .join(PRE_REPLACE_FAILURE_MARKER)
    }

    fn record_path(&self, repo_id: RepoId) -> PathBuf {
        self.dir.join(format!("{repo_id}.json"))
    }
}

pub(super) struct RepoCatalogStoreGuard<'a> {
    file: &'a File,
}

impl Drop for RepoCatalogStoreGuard<'_> {
    fn drop(&mut self) {
        if let Err(error) = unlock_file(self.file) {
            tracing::error!(%error, "failed to unlock repo catalog authority file");
        }
    }
}

fn before_replace_failure(
    primary: std::io::Error,
    temp: Option<&Path>,
) -> RepoCatalogPublishFailure {
    let cleanup = temp.and_then(|temp| match std::fs::remove_file(temp) {
        Ok(()) => None,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => Some(error),
    });
    RepoCatalogPublishFailure {
        phase: RepoCatalogPublishPhase::BeforeReplace,
        primary,
        cleanup,
    }
}

fn is_owned_temp_name(name: &str) -> bool {
    let Some(body) = name
        .strip_prefix(TEMP_PREFIX)
        .and_then(|name| name.strip_suffix(".tmp"))
    else {
        return false;
    };
    let mut parts = body.split('.');
    let Some(repo_id) = parts.next() else {
        return false;
    };
    let Some(process_id) = parts.next() else {
        return false;
    };
    let Some(nonce) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && Uuid::parse_str(repo_id).is_ok()
        && !process_id.is_empty()
        && process_id.bytes().all(|byte| byte.is_ascii_digit())
        && Uuid::parse_str(nonce).is_ok()
}

fn validate_regular_entry(
    entry: &std::fs::DirEntry,
    max_bytes: u64,
    context: &str,
) -> Result<(), RepoCatalogError> {
    let file_type = entry.file_type()?;
    let metadata = std::fs::symlink_metadata(entry.path())?;
    if !file_type.is_file()
        || file_type.is_symlink()
        || !metadata.is_file()
        || is_reparse(&metadata)
    {
        return Err(RepoCatalogError::InvalidRecord(format!(
            "unexpected non-regular {context}: {:?}",
            entry.path()
        )));
    }
    if metadata.len() > max_bytes {
        return Err(RepoCatalogError::InvalidRecord(format!(
            "{context} exceeds {max_bytes} bytes: {:?}",
            entry.path()
        )));
    }
    Ok(())
}

fn read_record(
    mut file: File,
    path: &Path,
    expected_repo_id: RepoId,
) -> Result<RepoCatalogMembershipRecord, RepoCatalogError> {
    let metadata = file.metadata()?;
    if metadata.len() > CATALOG_RECORD_MAX_BYTES {
        return Err(RepoCatalogError::InvalidRecord(format!(
            "repo catalog record exceeds {CATALOG_RECORD_MAX_BYTES} bytes: {path:?}"
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(CATALOG_RECORD_MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > CATALOG_RECORD_MAX_BYTES {
        return Err(RepoCatalogError::InvalidRecord(format!(
            "repo catalog record exceeded {CATALOG_RECORD_MAX_BYTES} bytes while reading: {path:?}"
        )));
    }
    let record: RepoCatalogMembershipRecord = serde_json::from_slice(&bytes)?;
    record.validate(expected_repo_id)?;
    if bytes != canonical_record_bytes(&record)? {
        return Err(RepoCatalogError::InvalidRecord(format!(
            "repo catalog record is not deterministic JSON v2: {path:?}"
        )));
    }
    Ok(record)
}

fn canonical_record_bytes(
    record: &RepoCatalogMembershipRecord,
) -> Result<Vec<u8>, RepoCatalogError> {
    let mut bytes = serde_json::to_vec(record)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn ensure_regular_directory(
    path: &Path,
    context: &str,
    create: bool,
) -> Result<(), RepoCatalogError> {
    if create {
        match std::fs::create_dir(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("failed to inspect {context} at {path:?}: {error}"),
        )
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(RepoCatalogError::InvalidRecord(format!(
            "{context} is not a regular directory: {path:?}"
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
