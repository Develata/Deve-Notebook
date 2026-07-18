use deve_core::codec;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{
    REDB_SCHEMA_VERSION, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
};
use tempfile::TempDir;

const REMOTE_IMPORT_SESSIONS: redb::TableDefinition<u128, &[u8]> =
    redb::TableDefinition::new("remote_import_sessions");
const REMOTE_IMPORT_RUNTIME: redb::TableDefinition<u8, &[u8]> =
    redb::TableDefinition::new("remote_import_runtime");
const PROJECTION_FAULTS: redb::TableDefinition<[u8; 32], &[u8]> =
    redb::TableDefinition::new("projection_faults");

#[test]
fn init_fails_closed_when_existing_local_repo_lacks_metadata_table() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir).expect("create local dir");
    let repo_id = uuid::Uuid::new_v4();
    redb::Database::create(local_dir.join(format!("{repo_id}.redb"))).expect("create db");

    let err = match RepoManager::init(&ledger_dir, 8, None, None) {
        Ok(_) => panic!("missing repo metadata table must fail init"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("schema version missing"));
}

#[test]
fn init_fails_closed_when_existing_local_repo_lacks_metadata_value() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir).expect("create local dir");
    let repo_id = uuid::Uuid::new_v4();
    let db = redb::Database::create(local_dir.join(format!("{repo_id}.redb"))).expect("create db");
    let txn = db.begin_write().expect("write txn");
    {
        let mut table = txn.open_table(REPO_METADATA).expect("repo metadata");
        table
            .insert(
                &REPO_SCHEMA_VERSION_METADATA_KEY,
                codec::encode(&REDB_SCHEMA_VERSION)
                    .expect("encode schema version")
                    .as_slice(),
            )
            .expect("insert placeholder");
        let _ = txn
            .open_table(REMOTE_IMPORT_SESSIONS)
            .expect("remote import sessions");
        let _ = txn
            .open_table(REMOTE_IMPORT_RUNTIME)
            .expect("remote import runtime");
        let _ = txn
            .open_table(PROJECTION_FAULTS)
            .expect("projection faults");
    }
    txn.commit().expect("commit");
    drop(db);

    let err = match RepoManager::init(&ledger_dir, 8, None, None) {
        Ok(_) => panic!("missing repo metadata row must fail init"),
        Err(err) => err,
    };
    assert!(
        err.to_string().contains("repository metadata missing"),
        "{err:#}"
    );
}
