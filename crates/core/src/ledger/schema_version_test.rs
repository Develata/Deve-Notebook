use crate::codec;
use crate::ledger::schema::{
    REDB_SCHEMA_VERSION, REMOTE_IMPORT_RUNTIME, REMOTE_IMPORT_SESSIONS, REPO_INFO_METADATA_KEY,
    REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
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
    let version: u16 = codec::decode(raw.value())?;

    assert_eq!(version, REDB_SCHEMA_VERSION);
    assert_eq!(version, 4);
    Ok(())
}

#[test]
fn redb_v3_repo_fails_closed_without_rewriting_or_creating_v4_tables() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir)?;
    let repo_id = uuid::Uuid::new_v4();
    let path = local_dir.join(format!("{repo_id}.redb"));
    let db = redb::Database::create(&path)?;
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = codec::encode(&3u16)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        let info = RepoInfo {
            uuid: repo_id,
            name: "default".into(),
            url: Some("urn:default".into()),
        };
        let bytes = codec::encode(&info)?;
        table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
    }
    txn.commit()?;
    drop(db);

    let err = match RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default")) {
        Ok(_) => panic!("v3 repo must fail closed"),
        Err(error) => error,
    };
    assert!(err.to_string().contains("expected 4"));

    let db = crate::ledger::database::cached_database(&path)?;
    let read = db.begin_read()?;
    let metadata = read.open_table(REPO_METADATA)?;
    let raw = metadata
        .get(&REPO_SCHEMA_VERSION_METADATA_KEY)?
        .expect("v3 stamp remains");
    let version: u16 = codec::decode(raw.value())?;
    assert_eq!(version, 3);
    assert!(matches!(
        read.open_table(REMOTE_IMPORT_SESSIONS),
        Err(redb::TableError::TableDoesNotExist(_))
    ));
    assert!(matches!(
        read.open_table(REMOTE_IMPORT_RUNTIME),
        Err(redb::TableError::TableDoesNotExist(_))
    ));
    Ok(())
}

#[test]
fn shadow_v4_uses_uuid_stem_without_local_remote_import_tables() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default"))?;
    let peer = crate::models::PeerId::new("peer-a");
    let repo_id = uuid::Uuid::new_v4();
    repo.ensure_shadow_db(&peer, &repo_id)?;
    let path = ledger_dir
        .join("remotes")
        .join(peer.to_filename())
        .join(format!("{repo_id}.redb"));
    assert!(path.is_file());
    let db = crate::ledger::database::cached_database(&path)?;
    let read = db.begin_read()?;
    let metadata = read.open_table(REPO_METADATA)?;
    let raw = metadata
        .get(&REPO_SCHEMA_VERSION_METADATA_KEY)?
        .expect("schema version");
    let version: u16 = codec::decode(raw.value())?;
    assert_eq!(version, 4);
    assert!(matches!(
        read.open_table(REMOTE_IMPORT_SESSIONS),
        Err(redb::TableError::TableDoesNotExist(_))
    ));
    assert!(matches!(
        read.open_table(REMOTE_IMPORT_RUNTIME),
        Err(redb::TableError::TableDoesNotExist(_))
    ));
    Ok(())
}

#[test]
fn uuid_first_repo_reopens_by_display_name_without_duplicate_file() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let first = RepoManager::init(&ledger_dir, 8, Some("notes"), Some("urn:notes"))?;
    let first_info = first.get_repo_info()?.expect("first info");
    let first_stem = first.local_repo_name().to_string();
    drop(first);

    let reopened = RepoManager::init(&ledger_dir, 8, Some("notes"), Some("urn:notes"))?;
    assert_eq!(reopened.local_repo_name(), first_stem);
    assert_eq!(
        reopened.get_repo_info()?.expect("reopened info"),
        first_info
    );
    let files = std::fs::read_dir(ledger_dir.join("local"))?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(files.len(), 1);
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
        let bytes = codec::encode(&info)?;
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
        let version = codec::encode(&bad_version)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        let info = RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "default".into(),
            url: Some("urn:default".into()),
        };
        let bytes = codec::encode(&info)?;
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
