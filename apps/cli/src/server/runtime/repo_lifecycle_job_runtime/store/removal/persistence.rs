//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Durable removal-record load and atomic publication.

#[cfg(test)]
use super::PRE_REPLACE_FAILURE_MARKER;
use super::RemovalPreparationRecord;
use crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError;
use crate::server::runtime::repo_lifecycle_job_runtime::store::{
    RECEIPT_MAX_BYTES, checked_directory, is_reparse, store_invalid,
};
use deve_core::utils::fs as safe_fs;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;

const STORE_ENTRY_LIMIT: usize = 2_048;
const STORE_AGGREGATE_MAX_BYTES: u64 = 16 * 1024 * 1024;

pub(in crate::server::runtime::repo_lifecycle_job_runtime::store) fn load_removals(
    dir: &Path,
) -> Result<BTreeMap<Uuid, RemovalPreparationRecord>, RepoLifecycleJobError> {
    let dir = checked_directory(dir, true)?;
    let mut records = BTreeMap::new();
    let mut entry_count = 0_usize;
    let mut aggregate_bytes = 0_u64;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        entry_count = entry_count.saturating_add(1);
        aggregate_bytes = aggregate_bytes.saturating_add(metadata.len());
        if entry_count > STORE_ENTRY_LIMIT || aggregate_bytes > STORE_AGGREGATE_MAX_BYTES {
            return Err(store_invalid("removal store exceeds bounded load budget"));
        }
        if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
            return Err(store_invalid("removal store contains a non-regular entry"));
        }
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let preparation_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| store_invalid("removal record name is not a preparation UUID"))?;
        let record = read_removal(&path)?;
        if record.preparation_id != preparation_id
            || records.insert(preparation_id, record).is_some()
        {
            return Err(store_invalid("duplicate or mismatched removal preparation"));
        }
    }
    Ok(records)
}

pub(in crate::server::runtime::repo_lifecycle_job_runtime::store) fn publish_removal(
    dir: &Path,
    record: &RemovalPreparationRecord,
) -> Result<(), RepoLifecycleJobError> {
    record.validate()?;
    let path = dir.join(format!("{}.json", record.preparation_id));
    let temp = dir.join(format!(
        ".{}.{}.{}.tmp",
        record.preparation_id,
        std::process::id(),
        Uuid::new_v4()
    ));
    let mut bytes = serde_json::to_vec_pretty(record)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > RECEIPT_MAX_BYTES {
        return Err(store_invalid("removal preparation exceeds size budget"));
    }
    let result = (|| -> Result<(), RepoLifecycleJobError> {
        let mut file = safe_fs::create_atomic_replace_temp(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        #[cfg(test)]
        if dir.join(PRE_REPLACE_FAILURE_MARKER).try_exists()? {
            return Err(store_invalid(
                "injected removal preparation pre-replace failure",
            ));
        }
        safe_fs::replace_file_atomically(&file, &temp, &path)?;
        safe_fs::sync_directory(dir)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn read_removal(path: &Path) -> Result<RemovalPreparationRecord, RepoLifecycleJobError> {
    let file = safe_fs::open_regular_file_read(path, "repo removal preparation")?;
    let metadata = file.metadata()?;
    if metadata.len() > RECEIPT_MAX_BYTES {
        return Err(store_invalid("removal preparation exceeds size budget"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(RECEIPT_MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > RECEIPT_MAX_BYTES {
        return Err(store_invalid(
            "removal preparation exceeds read size budget",
        ));
    }
    let record: RemovalPreparationRecord = serde_json::from_slice(&bytes)?;
    record.validate()?;
    Ok(record)
}
