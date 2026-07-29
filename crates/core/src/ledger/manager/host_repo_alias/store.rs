//! plan_ref:
//!   - 04_repository#host-repo-alias-contract

use super::model::{
    HostRepoAliasBinding, HostRepoAliasError, HostRepoAliasSetResult, normalize_alias,
};
use crate::models::RepoId;
use crate::utils::fs::{
    create_atomic_replace_temp, ensure_open_file_matches_path, lock_file_exclusive,
    open_regular_file_lock, open_regular_file_read, replace_file_atomically, sync_directory,
};
use crate::utils::notegit;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const STORE_FILE: &str = "repo-aliases.json";
const LOCK_FILE: &str = "repo-aliases.lock";
const STORE_FORMAT: &str = "deve.host-repo-alias-store";
const STORE_VERSION: u32 = 1;
const STORE_MAX_BYTES: u64 = 2 * 1024 * 1024;
#[cfg(test)]
pub(super) const PRE_REPLACE_FAILURE_MARKER: &str = ".repo-alias-fail-before-replace";
#[cfg(test)]
pub(super) const POST_REPLACE_FAILURE_MARKER: &str = ".repo-alias-fail-after-replace";

#[derive(Debug, Default)]
pub(super) struct AliasStore {
    rows: BTreeMap<RepoId, HostRepoAliasBinding>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AliasStoreDocument {
    format: String,
    version: u32,
    aliases: Vec<HostRepoAliasBinding>,
}

impl AliasStore {
    pub(super) fn load(ledger_dir: &Path) -> Result<Self, HostRepoAliasError> {
        let path = store_path(ledger_dir);
        let file = match open_regular_file_read(&path, "host repo alias store") {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(HostRepoAliasError::StoreInvalid(error.to_string())),
        };
        let metadata = file.metadata()?;
        if metadata.len() > STORE_MAX_BYTES {
            return Err(HostRepoAliasError::StoreInvalid(format!(
                "store exceeds {STORE_MAX_BYTES} bytes"
            )));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(STORE_MAX_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > STORE_MAX_BYTES {
            return Err(HostRepoAliasError::StoreInvalid(format!(
                "store exceeds {STORE_MAX_BYTES} bytes while reading"
            )));
        }
        let document: AliasStoreDocument = serde_json::from_slice(&bytes)
            .map_err(|error| HostRepoAliasError::StoreInvalid(error.to_string()))?;
        if document.format != STORE_FORMAT {
            return Err(HostRepoAliasError::StoreInvalid(format!(
                "unsupported format {}",
                document.format
            )));
        }
        if document.version != STORE_VERSION {
            return Err(HostRepoAliasError::StoreInvalid(format!(
                "unsupported version {}",
                document.version
            )));
        }

        let mut rows = BTreeMap::new();
        for binding in document.aliases {
            let normalized = normalize_alias(&binding.alias)
                .map_err(|error| HostRepoAliasError::StoreInvalid(error.to_string()))?;
            if normalized != binding.alias {
                return Err(HostRepoAliasError::StoreInvalid(format!(
                    "alias for {} is not normalized",
                    binding.repo_id
                )));
            }
            if binding.alias_revision == 0 {
                return Err(HostRepoAliasError::StoreInvalid(format!(
                    "binding for {} has revision zero",
                    binding.repo_id
                )));
            }
            let repo_id = binding.repo_id;
            if rows.insert(repo_id, binding).is_some() {
                return Err(HostRepoAliasError::StoreInvalid(format!(
                    "duplicate RepoId {repo_id}"
                )));
            }
        }
        Ok(Self { rows })
    }

    pub(super) fn bindings(&self) -> impl Iterator<Item = &HostRepoAliasBinding> {
        self.rows.values()
    }

    pub(super) fn binding_or_fallback(&self, repo_id: RepoId) -> HostRepoAliasBinding {
        self.rows
            .get(&repo_id)
            .cloned()
            .unwrap_or_else(|| HostRepoAliasBinding {
                repo_id,
                alias: repo_id.to_string(),
                alias_revision: 0,
            })
    }

    pub(super) fn set(
        &mut self,
        repo_id: RepoId,
        alias: String,
        expected_alias_revision: u64,
    ) -> Result<HostRepoAliasSetResult, HostRepoAliasError> {
        let current = self.binding_or_fallback(repo_id);
        if current.alias_revision != expected_alias_revision {
            return Err(HostRepoAliasError::RevisionConflict {
                repo_id,
                expected: expected_alias_revision,
                current: current.alias_revision,
            });
        }
        if current.alias_revision != 0 && current.alias == alias {
            return Ok(HostRepoAliasSetResult {
                binding: current,
                changed: false,
            });
        }
        let alias_revision = current
            .alias_revision
            .checked_add(1)
            .ok_or(HostRepoAliasError::RevisionOverflow(repo_id))?;
        let binding = HostRepoAliasBinding {
            repo_id,
            alias,
            alias_revision,
        };
        self.rows.insert(repo_id, binding.clone());
        Ok(HostRepoAliasSetResult {
            binding,
            changed: true,
        })
    }

    pub(super) fn remove_exact(&mut self, expected: &HostRepoAliasBinding) -> bool {
        self.rows.remove(&expected.repo_id).is_some()
    }

    pub(super) fn publish(&self, ledger_dir: &Path) -> Result<(), HostRepoAliasError> {
        let host_dir = checked_host_dir(ledger_dir)?;
        let path = host_dir.join(STORE_FILE);
        let temp = host_dir.join(format!(
            ".{STORE_FILE}.{}.{}.tmp",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let document = AliasStoreDocument {
            format: STORE_FORMAT.to_owned(),
            version: STORE_VERSION,
            aliases: self.rows.values().cloned().collect(),
        };
        let mut bytes = serde_json::to_vec_pretty(&document)?;
        bytes.push(b'\n');
        if bytes.len() as u64 > STORE_MAX_BYTES {
            return Err(HostRepoAliasError::StoreInvalid(format!(
                "serialized store exceeds {STORE_MAX_BYTES} bytes"
            )));
        }

        let write_result = (|| -> Result<(), HostRepoAliasError> {
            let mut file = create_atomic_replace_temp(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            #[cfg(test)]
            if host_dir.join(PRE_REPLACE_FAILURE_MARKER).try_exists()? {
                return Err(std::io::Error::other(
                    "injected repo alias failure before atomic replace",
                )
                .into());
            }
            replace_file_atomically(&file, &temp, &path)?;
            #[cfg(test)]
            if host_dir.join(POST_REPLACE_FAILURE_MARKER).try_exists()? {
                return Err(std::io::Error::other(
                    "injected repo alias failure after atomic replace",
                )
                .into());
            }
            sync_directory(&host_dir)?;
            Ok(())
        })();
        if write_result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        write_result
    }
}

pub(super) struct AliasStoreGuard {
    _file: std::fs::File,
}

impl AliasStoreGuard {
    pub(super) fn acquire(ledger_dir: &Path) -> Result<Self, HostRepoAliasError> {
        let host_dir = checked_host_dir(ledger_dir)?;
        let path = host_dir.join(LOCK_FILE);
        let file = open_regular_file_lock(&path, "host repo alias lock")
            .map_err(|error| HostRepoAliasError::StoreInvalid(error.to_string()))?;
        lock_file_exclusive(&file)?;
        ensure_open_file_matches_path(&file, &path, "host repo alias lock")
            .map_err(|error| HostRepoAliasError::StoreInvalid(error.to_string()))?;
        Ok(Self { _file: file })
    }
}

fn store_path(ledger_dir: &Path) -> PathBuf {
    notegit::host_dir(ledger_dir).join(STORE_FILE)
}

fn checked_host_dir(ledger_dir: &Path) -> Result<PathBuf, HostRepoAliasError> {
    let host_dir = notegit::host_dir(ledger_dir);
    std::fs::create_dir_all(&host_dir)?;
    let metadata = std::fs::symlink_metadata(&host_dir)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(HostRepoAliasError::StoreInvalid(format!(
            "host runtime path is not a regular directory: {host_dir:?}"
        )));
    }
    Ok(host_dir)
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
