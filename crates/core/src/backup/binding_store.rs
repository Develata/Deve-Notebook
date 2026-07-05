//! plan_ref:
//!   - 06_backup#backup-branch-binding-contract
//!
//! Host-local backup binding persistence.
//!
//! This module stores only secret-free locator and branch binding metadata
//! under the host runtime directory. It does not append ledger facts, persist
//! credentials or key material, call remote providers, write source-control
//! state, or touch Projection Workspaces.

use super::binding::{BackupBindingError, BackupBranchBinding, validate_backup_branch_bindings};
use super::locator::{BackupLocator, BackupLocatorError};
use crate::utils::notegit;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[cfg(test)]
mod tests;

const BINDING_STORE_VERSION: u32 = 1;
const BINDING_STORE_FILE: &str = "backup-bindings.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBindingRecord {
    pub bound_at_unix_ms: i64,
    pub locator: BackupLocator,
    pub binding: BackupBranchBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BackupBindingFile {
    version: u32,
    #[serde(default)]
    bindings: Vec<BackupBindingRecord>,
}

impl Default for BackupBindingFile {
    fn default() -> Self {
        Self {
            version: BINDING_STORE_VERSION,
            bindings: Vec::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum BackupBindingStoreError {
    #[error("failed to {action} backup binding store {path:?}: {source}")]
    Io {
        action: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse backup binding store {path:?}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to serialize backup binding store: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("unsupported backup binding store version {version} in {path:?}")]
    UnsupportedVersion { version: u32, path: PathBuf },
    #[error("backup binding store record path does not match its locator")]
    LocatorBindingMismatch,
    #[error("backup writable remote prefix is already bound to an active writer")]
    DuplicateWritablePhysicalPath,
    #[error("backup binding store path is not a regular host-local file: {0:?}")]
    NonRegularStorePath(PathBuf),
    #[error("backup binding does not exist")]
    MissingBinding,
    #[error(transparent)]
    Binding(#[from] BackupBindingError),
    #[error(transparent)]
    Locator(#[from] BackupLocatorError),
}

pub fn backup_binding_store_path_for(ledger_dir: &Path) -> PathBuf {
    notegit::host_dir(ledger_dir).join(BINDING_STORE_FILE)
}

pub fn list_backup_binding_records(
    ledger_dir: &Path,
) -> Result<Vec<BackupBindingRecord>, BackupBindingStoreError> {
    let file = read_backup_binding_file(&backup_binding_store_path_for(ledger_dir))?;
    validate_backup_binding_records(&file.bindings)?;
    Ok(file.bindings)
}

pub fn persist_backup_branch_binding(
    ledger_dir: &Path,
    locator: BackupLocator,
    binding: BackupBranchBinding,
) -> Result<BackupBindingRecord, BackupBindingStoreError> {
    let path = backup_binding_store_path_for(ledger_dir);
    let mut file = read_backup_binding_file(&path)?;
    let record = BackupBindingRecord {
        bound_at_unix_ms: chrono::Utc::now().timestamp_millis(),
        locator,
        binding,
    };

    file.bindings
        .retain(|item| !same_binding_key(item, &record.binding));
    file.bindings.push(record.clone());
    sort_records(&mut file.bindings);
    validate_backup_binding_records(&file.bindings)?;
    write_backup_binding_file(&path, &file)?;
    Ok(record)
}

pub fn remove_backup_branch_binding(
    ledger_dir: &Path,
    locator: &BackupLocator,
    binding: &BackupBranchBinding,
) -> Result<BackupBindingRecord, BackupBindingStoreError> {
    let path = backup_binding_store_path_for(ledger_dir);
    let mut file = read_backup_binding_file(&path)?;
    validate_backup_binding_records(&file.bindings)?;
    let index = file
        .bindings
        .iter()
        .position(|item| item.locator == *locator && item.binding == *binding)
        .ok_or(BackupBindingStoreError::MissingBinding)?;
    let removed = file.bindings.remove(index);
    validate_backup_binding_records(&file.bindings)?;
    write_backup_binding_file(&path, &file)?;
    Ok(removed)
}

fn read_backup_binding_file(path: &Path) -> Result<BackupBindingFile, BackupBindingStoreError> {
    if !ensure_existing_store_is_regular(path)? {
        return Ok(BackupBindingFile::default());
    }

    let content = std::fs::read_to_string(path).map_err(|source| BackupBindingStoreError::Io {
        action: "read",
        path: path.to_path_buf(),
        source,
    })?;
    let file: BackupBindingFile =
        toml::from_str(&content).map_err(|source| BackupBindingStoreError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    if file.version != BINDING_STORE_VERSION {
        return Err(BackupBindingStoreError::UnsupportedVersion {
            version: file.version,
            path: path.to_path_buf(),
        });
    }
    Ok(file)
}

fn write_backup_binding_file(
    path: &Path,
    file: &BackupBindingFile,
) -> Result<(), BackupBindingStoreError> {
    let Some(parent) = path.parent() else {
        return Err(BackupBindingStoreError::Io {
            action: "locate parent for",
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "backup binding store path has no parent",
            ),
        });
    };
    let existed = ensure_existing_store_is_regular(path)?;
    std::fs::create_dir_all(parent).map_err(|source| BackupBindingStoreError::Io {
        action: "create parent for",
        path: path.to_path_buf(),
        source,
    })?;
    let content = toml::to_string_pretty(file)?;
    let temp_path = path.with_file_name(format!(
        ".{}.tmp-{}-{}",
        BINDING_STORE_FILE,
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    {
        let mut temp = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|source| BackupBindingStoreError::Io {
                action: "create temp for",
                path: temp_path.clone(),
                source,
            })?;
        temp.write_all(content.as_bytes())
            .map_err(|source| BackupBindingStoreError::Io {
                action: "write temp for",
                path: temp_path.clone(),
                source,
            })?;
    }
    if existed {
        ensure_existing_store_is_regular(path)?;
        std::fs::remove_file(path).map_err(|source| BackupBindingStoreError::Io {
            action: "replace",
            path: path.to_path_buf(),
            source,
        })?;
    }
    if let Err(source) = std::fs::rename(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(BackupBindingStoreError::Io {
            action: "rename temp into",
            path: path.to_path_buf(),
            source,
        });
    }
    Ok(())
}

fn validate_backup_binding_records(
    records: &[BackupBindingRecord],
) -> Result<(), BackupBindingStoreError> {
    let mut writable_physical_paths = HashSet::new();
    for record in records {
        let branch = record
            .locator
            .branch_locator(&record.binding.writer_identity)?;
        if branch.branch_path != record.binding.branch_path {
            return Err(BackupBindingStoreError::LocatorBindingMismatch);
        }
        if record.binding.access == super::binding::BackupBindingAccess::Writable {
            let physical_key = (
                record.locator.provider.protocol().to_string(),
                record.locator.endpoint.clone().unwrap_or_default(),
                record.locator.namespace.clone(),
                record.binding.branch_path.clone(),
            );
            if !writable_physical_paths.insert(physical_key) {
                return Err(BackupBindingStoreError::DuplicateWritablePhysicalPath);
            }
        }
    }
    let bindings = records
        .iter()
        .map(|record| record.binding.clone())
        .collect::<Vec<_>>();
    validate_backup_branch_bindings(&bindings)?;
    Ok(())
}

fn ensure_existing_store_is_regular(path: &Path) -> Result<bool, BackupBindingStoreError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_symlink() || !file_type.is_file() {
                return Err(BackupBindingStoreError::NonRegularStorePath(
                    path.to_path_buf(),
                ));
            }
            Ok(true)
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(BackupBindingStoreError::Io {
            action: "stat",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn sort_records(records: &mut [BackupBindingRecord]) {
    records.sort_by(|left, right| {
        (
            left.binding.repo_id,
            left.binding.branch_name.as_str(),
            left.binding.writer_identity.as_str(),
            left.binding.branch_path.as_str(),
        )
            .cmp(&(
                right.binding.repo_id,
                right.binding.branch_name.as_str(),
                right.binding.writer_identity.as_str(),
                right.binding.branch_path.as_str(),
            ))
    });
}

fn same_binding_key(record: &BackupBindingRecord, binding: &BackupBranchBinding) -> bool {
    record.binding.repo_id == binding.repo_id
        && record.binding.branch_name == binding.branch_name
        && record.binding.writer_identity == binding.writer_identity
}
