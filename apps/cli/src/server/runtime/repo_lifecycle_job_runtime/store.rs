//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#repo-lifecycle-coordinator

use super::model::RepoLifecycleJobError;
use deve_core::models::RepoId;
use deve_core::utils::{fs as safe_fs, notegit};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(super) mod removal;
pub(super) use removal::{RemovalPreparationRecord, RemovalPreparationState};
mod receipt;
pub(super) use receipt::LifecycleReceipt;
mod request_namespace;
#[cfg(test)]
mod test_support;
#[cfg(test)]
pub(super) use test_support::{removal_retention_removals_for_test, retention_removals_for_test};
mod retention;

const RECEIPT_DIR: &str = "repo-lifecycle-jobs";
const LOCK_FILE: &str = "repo-lifecycle-jobs.lock";
const RECEIPT_MAX_BYTES: u64 = 64 * 1024;
const TERMINAL_RETENTION_MS: i64 = 24 * 60 * 60 * 1_000;
const TERMINAL_RECEIPT_LIMIT: usize = 1024;
#[cfg(test)]
pub(super) const POST_REPLACE_FAILURE_MARKER: &str = ".inject-post-replace-failure";

pub(super) struct ReceiptStore {
    dir: PathBuf,
    rows: BTreeMap<Uuid, LifecycleReceipt>,
    removal_dir: PathBuf,
    removals: BTreeMap<Uuid, RemovalPreparationRecord>,
    _lock: std::fs::File,
}

impl ReceiptStore {
    pub(super) fn open(ledger_dir: &Path) -> Result<Self, RepoLifecycleJobError> {
        let host_dir = checked_directory(&notegit::host_dir(ledger_dir), true)?;
        let lock_path = host_dir.join(LOCK_FILE);
        let lock = safe_fs::open_regular_file_lock(&lock_path, "repo lifecycle job lock")?;
        lock.try_lock().map_err(|error| {
            store_invalid(format!("repo lifecycle job lock is already held: {error}"))
        })?;
        safe_fs::ensure_open_file_matches_path(&lock, &lock_path, "repo lifecycle job lock")?;
        let dir = checked_directory(&host_dir.join(RECEIPT_DIR), true)?;
        let removal_dir = checked_directory(&dir.join("removals"), true)?;
        let mut rows = BTreeMap::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)?;
            if path == removal_dir {
                continue;
            }
            if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
                return Err(store_invalid(
                    "receipt directory contains a non-regular entry",
                ));
            }
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let request_id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| Uuid::parse_str(value).ok())
                .ok_or_else(|| store_invalid("receipt file name is not a request UUID"))?;
            let receipt = read_receipt(&path, request_id)?;
            if rows.insert(request_id, receipt).is_some() {
                return Err(store_invalid("duplicate lifecycle request receipt"));
            }
        }
        let removals = removal::load_removals(&removal_dir)?;
        request_namespace::validate(&rows, &removals)?;
        let mut active_repos = std::collections::HashSet::new();
        for receipt in rows
            .values()
            .chain(
                removals
                    .values()
                    .filter_map(RemovalPreparationRecord::receipt),
            )
            .filter(|receipt| {
                !receipt.phase.is_terminal()
                    || removals.values().any(|record| {
                        record.receipt_for_request(receipt.request_id).is_some()
                            && record.has_committed_debt()
                    })
            })
        {
            if !active_repos.insert(receipt.target_repo_id) {
                return Err(store_invalid(
                    "multiple active receipts target the same RepoId",
                ));
            }
        }
        Ok(Self {
            dir,
            rows,
            removal_dir,
            removals,
            _lock: lock,
        })
    }

    pub(super) fn receipt(&self, request_id: Uuid) -> Option<&LifecycleReceipt> {
        self.rows.get(&request_id).or_else(|| {
            self.removals
                .values()
                .find_map(|record| record.receipt_for_request(request_id))
        })
    }

    pub(super) fn active_receipts(&self) -> Vec<LifecycleReceipt> {
        self.rows
            .values()
            .chain(
                self.removals
                    .values()
                    .filter_map(RemovalPreparationRecord::receipt),
            )
            .filter(|receipt| {
                !receipt.phase.is_terminal()
                    || self.removals.values().any(|record| {
                        record.receipt_for_request(receipt.request_id).is_some()
                            && record.has_committed_debt()
                    })
            })
            .cloned()
            .collect()
    }

    pub(super) fn pending_publications(&self) -> Vec<Uuid> {
        self.rows
            .values()
            .chain(
                self.removals
                    .values()
                    .filter_map(RemovalPreparationRecord::receipt),
            )
            .filter(|receipt| receipt.publication_pending)
            .map(|receipt| receipt.request_id)
            .collect()
    }

    pub(super) fn insert(
        &mut self,
        receipt: LifecycleReceipt,
    ) -> Result<(), RepoLifecycleJobError> {
        if self.request_id_is_bound(receipt.request_id) {
            return Err(store_invalid("duplicate lifecycle request receipt"));
        }
        self.publish(&receipt)?;
        self.rows.insert(receipt.request_id, receipt);
        Ok(())
    }

    pub(super) fn update(
        &mut self,
        request_id: Uuid,
        mutate: impl FnOnce(&mut LifecycleReceipt),
    ) -> Result<LifecycleReceipt, RepoLifecycleJobError> {
        if let Some(mut receipt) = self.rows.get(&request_id).cloned() {
            mutate(&mut receipt);
            receipt.validate(request_id)?;
            self.publish(&receipt)?;
            self.rows.insert(request_id, receipt.clone());
            return Ok(receipt);
        }
        let preparation_id = self
            .removals
            .iter()
            .find_map(|(preparation_id, record)| {
                record
                    .receipt_for_request(request_id)
                    .is_some()
                    .then_some(*preparation_id)
            })
            .ok_or(RepoLifecycleJobError::NotFound)?;
        let mut record = self
            .removals
            .get(&preparation_id)
            .cloned()
            .ok_or(RepoLifecycleJobError::NotFound)?;
        let receipt = record
            .receipt_mut_for_request(request_id)
            .ok_or(RepoLifecycleJobError::NotFound)?;
        mutate(receipt);
        let receipt = receipt.clone();
        record.validate()?;
        removal::publish_removal(&self.removal_dir, &record)?;
        self.removals.insert(preparation_id, record);
        Ok(receipt)
    }

    pub(super) fn prune_terminal(
        &mut self,
        mut retain_normal_create: impl FnMut(RepoId) -> bool,
    ) -> Result<usize, RepoLifecycleJobError> {
        let remove = retention::terminal_retention_removals(
            self.rows.values(),
            chrono::Utc::now().timestamp_millis(),
            &mut retain_normal_create,
        );
        for request_id in &remove {
            let path = self.dir.join(format!("{request_id}.json"));
            let metadata = std::fs::symlink_metadata(&path)?;
            if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
                return Err(store_invalid("refusing to prune a non-regular receipt"));
            }
            std::fs::remove_file(path)?;
            self.rows.remove(request_id);
        }
        if !remove.is_empty() {
            safe_fs::sync_directory(&self.dir)?;
        }
        let removal_count = removal::prune_removals(
            &self.removal_dir,
            &mut self.removals,
            chrono::Utc::now().timestamp_millis(),
        )?;
        Ok(remove.len() + removal_count)
    }

    pub(super) fn prune_removals_only(&mut self) -> Result<usize, RepoLifecycleJobError> {
        removal::prune_removals(
            &self.removal_dir,
            &mut self.removals,
            chrono::Utc::now().timestamp_millis(),
        )
    }

    fn publish(&self, receipt: &LifecycleReceipt) -> Result<(), RepoLifecycleJobError> {
        let path = self.dir.join(format!("{}.json", receipt.request_id));
        let temp = self.dir.join(format!(
            ".{}.{}.{}.tmp",
            receipt.request_id,
            std::process::id(),
            Uuid::new_v4()
        ));
        let mut bytes = serde_json::to_vec_pretty(receipt)?;
        bytes.push(b'\n');
        if bytes.len() as u64 > RECEIPT_MAX_BYTES {
            return Err(store_invalid("serialized receipt exceeds size budget"));
        }
        let result = (|| -> Result<(), RepoLifecycleJobError> {
            let mut file = safe_fs::create_atomic_replace_temp(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            safe_fs::replace_file_atomically(&file, &temp, &path)?;
            #[cfg(test)]
            if self.dir.join(POST_REPLACE_FAILURE_MARKER).try_exists()? {
                return Err(store_invalid("injected post-replace sync failure"));
            }
            safe_fs::sync_directory(&self.dir)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result
    }
}

fn read_receipt(path: &Path, request_id: Uuid) -> Result<LifecycleReceipt, RepoLifecycleJobError> {
    let file = safe_fs::open_regular_file_read(path, "repo lifecycle receipt")?;
    let metadata = file.metadata()?;
    if metadata.len() > RECEIPT_MAX_BYTES {
        return Err(store_invalid("receipt exceeds size budget"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(RECEIPT_MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > RECEIPT_MAX_BYTES {
        return Err(store_invalid("receipt exceeds read size budget"));
    }
    let receipt: LifecycleReceipt = serde_json::from_slice(&bytes)?;
    receipt.validate(request_id)?;
    Ok(receipt)
}

fn checked_directory(path: &Path, create: bool) -> Result<PathBuf, RepoLifecycleJobError> {
    if create {
        std::fs::create_dir_all(path)?;
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(store_invalid(
            "lifecycle runtime path is not a regular directory",
        ));
    }
    Ok(path.to_path_buf())
}

fn store_invalid(detail: impl Into<String>) -> RepoLifecycleJobError {
    RepoLifecycleJobError::Store(detail.into())
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
