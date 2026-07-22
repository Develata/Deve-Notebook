//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 04_repository#local-repo-removal-contract

use super::{LifecycleReceipt, RemovalPreparationRecord, removal, retention};
use deve_core::models::RepoId;
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn retention_removals_for_test(
    receipts: &[LifecycleReceipt],
    now_ms: i64,
    protected: &HashSet<RepoId>,
) -> Vec<Uuid> {
    retention::terminal_retention_removals(receipts.iter(), now_ms, &mut |repo_id| {
        protected.contains(&repo_id)
    })
}

pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn removal_retention_removals_for_test(
    records: &BTreeMap<Uuid, RemovalPreparationRecord>,
    now_ms: i64,
) -> Vec<Uuid> {
    removal::removal_retention_removals_for_test(records, now_ms)
}
