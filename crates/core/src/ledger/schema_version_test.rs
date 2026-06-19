use crate::ledger::schema::{
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
};
use crate::ledger::{RepoInfo, RepoManager};
use tempfile::TempDir;

#[test]
fn redb_schema_version_written_on_repo_init() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let repo = RepoManager::init(dir.path().join("ledger"), 8, None, None)?;
    let handle = repo.open_database(None, repo.local_repo_name())?;

    let read = handle.db.begin_read()?;
    let table = read.open_table(REPO_METADATA)?;
    let raw = table
        .get(&REPO_SCHEMA_VERSION_METADATA_KEY)?
        .expect("schema version should be present");
    let version: u16 = bincode::deserialize(raw.value())?;

    assert_eq!(version, REDB_SCHEMA_VERSION);
    Ok(())
}

#[test]
fn redb_schema_version_missing_fails_closed() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir)?;

    let db = redb::Database::create(local_dir.join("default.redb"))?;
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let info = RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "default".into(),
            url: Some("urn:default".into()),
        };
        let bytes = bincode::serialize(&info)?;
        table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
    }
    txn.commit()?;
    drop(db);

    let err = match RepoManager::init(&ledger_dir, 8, None, None) {
        Ok(_) => panic!("unversioned repo should fail closed"),
        Err(error) => error,
    };

    assert!(err.to_string().contains("schema version missing"));
    Ok(())
}

#[test]
fn redb_schema_version_mismatch_fails_closed() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir)?;

    let db = redb::Database::create(local_dir.join("default.redb"))?;
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let bad_version = REDB_SCHEMA_VERSION + 1;
        let version = bincode::serialize(&bad_version)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        let info = RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "default".into(),
            url: Some("urn:default".into()),
        };
        let bytes = bincode::serialize(&info)?;
        table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
    }
    txn.commit()?;
    drop(db);

    let err = match RepoManager::init(&ledger_dir, 8, None, None) {
        Ok(_) => panic!("mismatched schema version should fail closed"),
        Err(error) => error,
    };

    assert!(err.to_string().contains("Unsupported redb schema version"));
    Ok(())
}
