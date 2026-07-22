//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Bounded retention that never prunes committed cleanup debt.

use super::super::{TERMINAL_RECEIPT_LIMIT, TERMINAL_RETENTION_MS, is_reparse, store_invalid};
use super::{RemovalPreparationRecord, RemovalPreparationState};
use deve_core::utils::fs as safe_fs;
use std::collections::BTreeMap;
use std::path::Path;
use uuid::Uuid;

pub(in super::super) fn prune_removals(
    dir: &Path,
    records: &mut BTreeMap<Uuid, RemovalPreparationRecord>,
    now_ms: i64,
) -> Result<usize, crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError> {
    let remove = removal_retention_removals(records, now_ms);
    for id in &remove {
        let path = dir.join(format!("{id}.json"));
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
            return Err(store_invalid(
                "refusing to prune a non-regular removal record",
            ));
        }
        std::fs::remove_file(path)?;
        records.remove(id);
    }
    if !remove.is_empty() {
        safe_fs::sync_directory(dir)?;
    }
    Ok(remove.len())
}

fn removal_retention_removals(
    records: &BTreeMap<Uuid, RemovalPreparationRecord>,
    now_ms: i64,
) -> Vec<Uuid> {
    let cutoff = now_ms.saturating_sub(TERMINAL_RETENTION_MS);
    let mut candidates = records
        .values()
        .filter(|record| match &record.state {
            RemovalPreparationState::Prepared { .. } => record.expires_at_unix_ms < now_ms,
            RemovalPreparationState::Superseded => true,
            RemovalPreparationState::ExecuteAdmitted {
                receipt, execution, ..
            } => {
                !execution.has_committed_debt()
                    && receipt.phase.is_terminal()
                    && !receipt.publication_pending
            }
        })
        .map(|record| (record.preparation_id, record.updated_at_ms))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, (id, updated_at_ms))| {
            (index >= TERMINAL_RECEIPT_LIMIT || updated_at_ms < cutoff).then_some(id)
        })
        .collect()
}

#[cfg(test)]
pub(in super::super) fn removal_retention_removals_for_test(
    records: &BTreeMap<Uuid, RemovalPreparationRecord>,
    now_ms: i64,
) -> Vec<Uuid> {
    removal_retention_removals(records, now_ms)
}
