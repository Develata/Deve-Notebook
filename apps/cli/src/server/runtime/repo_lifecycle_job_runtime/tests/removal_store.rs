//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Bounded retention and fail-closed loading for durable removal admission.

use super::super::RepoLifecycleJobError;
use super::super::removal::RepoRemovalIssuerBinding;
use super::super::store::{
    ReceiptStore, RemovalPreparationRecord, RemovalPreparationState,
    removal_retention_removals_for_test,
};
use deve_core::protocol::{LocalRepoRemovalBlocker, LocalRepoRemovalPreview};
use std::collections::BTreeMap;
use uuid::Uuid;

#[test]
fn removal_retention_is_bounded_before_loading_another_prepare() {
    let now = 50_000_i64;
    let mut records = BTreeMap::new();
    for index in 0..1_026_u128 {
        let preparation_id = Uuid::from_u128(index + 1);
        let mut record = RemovalPreparationRecord::prepared(
            Uuid::from_u128(index + 10_000),
            preparation_id,
            Uuid::from_u128(index + 20_000),
            1,
            None,
            RepoRemovalIssuerBinding::Web {
                principal_digest: "a".repeat(64),
                connection_epoch: 38,
            },
            Uuid::from_u128(30_000),
            None,
            None,
            LocalRepoRemovalPreview {
                deleted: Vec::new(),
                preserved: Vec::new(),
                warnings: Vec::new(),
                blockers: vec![LocalRepoRemovalBlocker::RepairRequired],
            },
            None,
            None,
            now + 1_000,
        );
        record.state = RemovalPreparationState::Superseded;
        record.updated_at_ms = now + index as i64;
        records.insert(preparation_id, record);
    }
    assert_eq!(removal_retention_removals_for_test(&records, now).len(), 2);
}

#[test]
fn removal_store_rejects_aggregate_bytes_over_load_budget() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let removal_dir = dir.path().join(".host/repo-lifecycle-jobs/removals");
    std::fs::create_dir_all(&removal_dir)?;
    std::fs::File::create(removal_dir.join("crash.tmp"))?.set_len(16 * 1024 * 1024 + 1)?;
    let error = match ReceiptStore::open(dir.path()) {
        Ok(_) => panic!("oversized durable removal store must fail closed"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        RepoLifecycleJobError::Store(detail) if detail.contains("bounded load budget")
    ));
    Ok(())
}
