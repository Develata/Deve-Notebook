//! plan_ref:
//!   - 03_storage/projection#durable-projection-fault-contract
//!   - 04_repository#repo-health-and-repair
//!
//! Repo-local durable recovery evidence for projection side effects.
//!
//! This module owns only the versioned fault model and narrow Redb primitives. It never writes
//! Ledger facts or Remote Import workflow rows; callers that need cross-table atomicity pass the
//! same `WriteTransaction` to `record_prepared_in_txn`.

mod store;
mod types;

#[cfg(test)]
pub(crate) use store::remote_import_origins_for_test;
pub(crate) use store::{
    clear_faults_for_repo, load_degraded_repo_ids, prepare_remote_import_fault, record_fault,
    record_prepared_in_txn,
};
pub(crate) use types::{PreparedProjectionFault, ProjectionFaultInput, ProjectionFaultKind};
