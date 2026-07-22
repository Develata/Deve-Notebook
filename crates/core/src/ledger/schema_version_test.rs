use crate::codec;
use crate::ledger::RepoManager;
use crate::ledger::schema::{
    PROJECTION_FAULTS, REDB_SCHEMA_VERSION, REMOTE_IMPORT_RUNTIME, REMOTE_IMPORT_SESSIONS,
    REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
};
use tempfile::TempDir;

#[test]
fn redb_schema_version_written_on_repo_init() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let (repo, repo_id) = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
    )?;
    let handle = repo.local_authority_lease_for_test(repo_id)?;

    let read = handle.db().begin_read()?;
    let table = read.open_table(REPO_METADATA)?;
    let raw = table
        .get(&REPO_SCHEMA_VERSION_METADATA_KEY)?
        .expect("schema version should be present");
    let version: u16 = codec::decode(raw.value())?;

    assert_eq!(version, REDB_SCHEMA_VERSION);
    assert_eq!(version, 4);
    read.open_table(PROJECTION_FAULTS)?;
    Ok(())
}

#[test]
fn redb_v3_repo_fails_closed_without_rewriting_or_creating_v4_tables() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let (repo, repo_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    let path = ledger_dir.join("local").join(format!("{repo_id}.redb"));
    let handle = repo.local_authority_lease_for_test(repo_id)?;
    let txn = handle.db().begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = codec::encode(&3u16)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
    }
    txn.delete_table(REMOTE_IMPORT_SESSIONS)?;
    txn.delete_table(REMOTE_IMPORT_RUNTIME)?;
    txn.delete_table(PROJECTION_FAULTS)?;
    txn.commit()?;
    drop(handle);
    drop(repo);

    let err = match RepoManager::init(&ledger_dir, 8, None, None) {
        Ok(_) => panic!("v3 repo must fail closed"),
        Err(error) => error,
    };
    assert!(err.to_string().contains("expected 4"));

    let db = crate::ledger::database::cached_shadow_database(&path)?;
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
    assert!(matches!(
        read.open_table(PROJECTION_FAULTS),
        Err(redb::TableError::TableDoesNotExist(_))
    ));
    Ok(())
}

#[test]
fn shadow_v4_uses_uuid_stem_without_local_remote_import_tables() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let (repo, _local_repo_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    let peer = crate::models::PeerId::new("peer-a");
    let repo_id = uuid::Uuid::new_v4();
    repo.ensure_shadow_db(&peer, &repo_id)?;
    let path = ledger_dir
        .join("remotes")
        .join(peer.to_filename())
        .join(format!("{repo_id}.redb"));
    assert!(path.is_file());
    repo.run_on_shadow_repo_by_id(&peer, &repo_id, |db| {
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
        assert!(matches!(
            read.open_table(PROJECTION_FAULTS),
            Err(redb::TableError::TableDoesNotExist(_))
        ));
        Ok(())
    })?;
    Ok(())
}

#[test]
fn local_v4_missing_projection_fault_table_fails_closed_without_repair() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let (repo, repo_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    let handle = repo.local_authority_lease_for_test(repo_id)?;
    let write = handle.db().begin_write()?;
    write.delete_table(PROJECTION_FAULTS)?;
    write.commit()?;

    let error = RepoManager::validate_local_repo_schema(handle.db())
        .expect_err("incomplete v4 local profile must fail closed");
    assert!(error.to_string().contains("projection_faults"));
    let read = handle.db().begin_read()?;
    assert!(matches!(
        read.open_table(PROJECTION_FAULTS),
        Err(redb::TableError::TableDoesNotExist(_))
    ));
    Ok(())
}

#[test]
fn uuid_first_repo_reopens_by_display_name_without_duplicate_file() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let (first, _repo_id) = crate::test_support::init_cataloged_repo_with_url(
        &ledger_dir,
        &dir.path().join("notes"),
        "urn:notes",
    )?;
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
    let (repo, repo_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    let handle = repo.local_authority_lease_for_test(repo_id)?;
    let txn = handle.db().begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        table.remove(&REPO_SCHEMA_VERSION_METADATA_KEY)?;
    }
    txn.commit()?;
    drop(handle);
    drop(repo);

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
    let (repo, repo_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    let handle = repo.local_authority_lease_for_test(repo_id)?;
    let txn = handle.db().begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let bad_version = REDB_SCHEMA_VERSION + 1;
        let version = codec::encode(&bad_version)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
    }
    txn.commit()?;
    drop(handle);
    drop(repo);

    let err = match RepoManager::init(&ledger_dir, 8, None, None) {
        Ok(_) => panic!("mismatched schema version should fail closed"),
        Err(error) => error,
    };

    assert!(err.to_string().contains("Unsupported redb schema version"));
    Ok(())
}
