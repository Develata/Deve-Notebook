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

#[test]
fn removal_preparation_v2_fails_closed() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let preparation_id = Uuid::new_v4();
    let record = RemovalPreparationRecord::prepared(
        Uuid::new_v4(),
        preparation_id,
        Uuid::new_v4(),
        1,
        None,
        RepoRemovalIssuerBinding::Web {
            principal_digest: "a".repeat(64),
            connection_epoch: 1,
        },
        Uuid::new_v4(),
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
        60_000,
    );
    let mut store = ReceiptStore::open(dir.path())?;
    store.publish_preparation(record)?;
    drop(store);

    let path = dir
        .path()
        .join(".host/repo-lifecycle-jobs/removals")
        .join(format!("{preparation_id}.json"));
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    value["version"] = serde_json::json!(2);
    std::fs::write(&path, serde_json::to_vec_pretty(&value)?)?;

    assert!(matches!(
        ReceiptStore::open(dir.path()),
        Err(RepoLifecycleJobError::Store(detail))
            if detail.contains("invalid removal preparation identity")
    ));
    Ok(())
}
