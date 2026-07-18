//! plan_ref:
//!   - 03_storage/projection#durable-projection-fault-contract

use super::*;
use crate::ledger::RepoManager;
use redb::ReadableTable;

#[test]
fn deterministic_key_and_repeat_upsert_preserve_first_seen() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let repo = RepoManager::init(temp.path().join("ledger"), 8, Some("main"), None)?;
    let info = repo.get_repo_info()?.expect("repo info");
    let first = prepare(
        &info,
        ProjectionFaultOrigin::ProjectionPersistence,
        ProjectionFaultKind::ProjectionWritebackFailed,
        Some("notes\\a.md"),
        None,
        Some(DocId::new()),
        Some(7),
        None,
        "first",
    );
    let mut second = prepare(
        &info,
        first.value.origin.clone(),
        first.value.fault_kind,
        first.value.target_path.as_deref(),
        None,
        first.value.doc_id,
        Some(8),
        None,
        "second",
    );
    second.value.last_seen_at_unix_ms = first.value.last_seen_at_unix_ms + 1;
    assert_eq!(first.key, second.key);

    let write = repo.local_db.begin_write()?;
    record_prepared_in_txn(&write, info.uuid, &first)?;
    record_prepared_in_txn(&write, info.uuid, &second)?;
    write.commit()?;

    let read = repo.local_db.begin_read()?;
    let table = read.open_table(PROJECTION_FAULTS)?;
    let row = table.get(&first.key)?.expect("fault row");
    let stored = decode_fault(first.key, row.value(), info.uuid)?;
    assert_eq!(
        stored.first_seen_at_unix_ms,
        first.value.first_seen_at_unix_ms
    );
    assert_eq!(
        stored.last_seen_at_unix_ms,
        second.value.last_seen_at_unix_ms
    );
    assert_eq!(stored.last_error, "second");
    assert_eq!(stored.retry_count, 2);
    assert_eq!(stored.target_path.as_deref(), Some("notes/a.md"));
    Ok(())
}

#[test]
fn corrupt_key_value_identity_blocks_load_and_clear() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let repo = RepoManager::init(temp.path().join("ledger"), 8, Some("main"), None)?;
    let info = repo.get_repo_info()?.expect("repo info");
    let repo_name = info.name.clone();
    let prepared =
        prepare_remote_import_fault(info.uuid, info.name, 1, 1, Uuid::new_v4(), 9, "failed");
    let wrong_key = [0x5a; 32];
    let bytes = crate::codec::encode(&prepared.value)?;
    let write = repo.local_db.begin_write()?;
    {
        write
            .open_table(PROJECTION_FAULTS)?
            .insert(&wrong_key, bytes.as_slice())?;
    }
    write.commit()?;

    let load_error = load_degraded_repo_ids(&repo).expect_err("bad key must fail closed");
    assert!(
        load_error
            .to_string()
            .contains("key/value identity mismatch")
    );
    let clear_error =
        clear_faults_for_repo(&repo, &repo_name).expect_err("repair clear must fail closed");
    assert!(
        clear_error
            .to_string()
            .contains("key/value identity mismatch")
    );
    let read = repo.local_db.begin_read()?;
    assert_eq!(read.open_table(PROJECTION_FAULTS)?.iter()?.count(), 1);
    Ok(())
}

#[test]
fn decoder_rejects_version_repo_and_trailing_or_malformed_payload() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let repo = RepoManager::init(temp.path().join("ledger"), 8, Some("main"), None)?;
    let info = repo.get_repo_info()?.expect("repo info");
    let prepared =
        prepare_remote_import_fault(info.uuid, info.name, 7, 3, Uuid::new_v4(), 11, "failed");

    let mut unsupported = prepared.value.clone();
    unsupported.value_version = PROJECTION_FAULT_VALUE_VERSION + 1;
    let unsupported_bytes = crate::codec::encode(&unsupported)?;
    assert!(
        decode_fault(prepared.key, &unsupported_bytes, info.uuid)
            .expect_err("unsupported value version")
            .to_string()
            .contains("unsupported Projection Fault value version")
    );

    let mut wrong_repo = prepared.value.clone();
    wrong_repo.repo_id = Uuid::new_v4();
    let wrong_repo_key = key_for(&wrong_repo);
    let wrong_repo_bytes = crate::codec::encode(&wrong_repo)?;
    assert!(
        decode_fault(wrong_repo_key, &wrong_repo_bytes, info.uuid)
            .expect_err("foreign RepoId")
            .to_string()
            .contains("differs from database RepoId")
    );

    let mut trailing = crate::codec::encode(&prepared.value)?;
    trailing.push(0);
    assert!(
        decode_fault(prepared.key, &trailing, info.uuid)
            .expect_err("trailing bytes")
            .to_string()
            .contains("trailing bytes")
    );
    assert!(decode_fault(prepared.key, &[0xff], info.uuid).is_err());
    Ok(())
}
