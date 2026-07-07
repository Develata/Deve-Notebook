use deve_core::codec;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{
    REDB_SCHEMA_VERSION, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
};
use tempfile::TempDir;

#[test]
fn init_fails_closed_when_existing_local_repo_lacks_metadata_table() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir).expect("create local dir");
    redb::Database::create(local_dir.join("default.redb")).expect("create db");

    let err = match RepoManager::init(&ledger_dir, 8, None, None) {
        Ok(_) => panic!("missing repo metadata table must fail init"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("repository metadata missing"));
}

#[test]
fn init_fails_closed_when_existing_local_repo_lacks_metadata_value() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir).expect("create local dir");
    let db = redb::Database::create(local_dir.join("default.redb")).expect("create db");
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
