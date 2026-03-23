use super::{RepoManager, cached_database};
use crate::ledger::{RepoInfo, schema::REPO_METADATA};
use crate::models::PeerId;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn create_secondary_repo(ledger_dir: &Path, name: &str, url: &str) {
    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir).expect("create local dir");
    let path = local_dir.join(format!("{name}.redb"));
    let db = redb::Database::create(&path).expect("create secondary db");
    crate::ledger::source_control::init_tables(&db).expect("init source control tables");
    let txn = db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA).expect("repo metadata");
    txn.commit().expect("commit metadata table");
    let txn = db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(
            &0,
            bincode::serialize(&RepoInfo {
                uuid: uuid::Uuid::new_v4(),
                name: name.into(),
                url: Some(url.into()),
            })
            .expect("serialize")
            .as_slice(),
        )
        .expect("write metadata");
    txn.commit().expect("commit metadata");
    drop(db);
}

#[cfg(unix)]
#[test]
fn cached_database_fails_closed_when_path_is_unstatable() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let blocked = dir.path().join("blocked");
    std::fs::create_dir_all(&blocked).expect("blocked dir");
    let original = std::fs::metadata(&blocked).expect("metadata").permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(&blocked, perms).expect("chmod 000");
    let path = blocked.join("notes.redb");

    let err = cached_database(&path).expect_err("unstatable path must fail closed");

    std::fs::set_permissions(&blocked, original).expect("restore perms");
    assert!(
        err.to_string().contains("Failed to stat database path")
            || err.to_string().contains("Permission denied")
    );
}

#[cfg(unix)]
#[test]
fn open_local_database_fails_closed_when_path_is_unstatable() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("repo");
    create_secondary_repo(&ledger_dir, "wiki", "urn:wiki");
    let local_dir = dir.path().join("ledger/local");
    let original = std::fs::metadata(&local_dir)
        .expect("metadata")
        .permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(&local_dir, perms).expect("chmod 000");

    let err = match repo.open_database(None, "wiki") {
        Ok(_) => panic!("unstatable local db path must fail closed"),
        Err(err) => err,
    };

    std::fs::set_permissions(&local_dir, original).expect("restore perms");
    assert!(
        err.to_string()
            .contains("Failed to stat local repo directory")
            || err
                .to_string()
                .contains("Failed to stat local database path")
            || err.to_string().contains("Permission denied")
    );
}

#[test]
fn open_local_database_fails_closed_when_repo_metadata_is_missing() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("repo");
    create_secondary_repo(&ledger_dir, "wiki", "urn:wiki");
    let wiki_db = repo.open_database(None, "wiki").expect("wiki db");
    let txn = wiki_db.db.begin_write().expect("write txn");
    txn.delete_table(REPO_METADATA).expect("delete metadata");
    txn.commit().expect("commit");

    let err = match repo.open_database(None, "wiki") {
        Ok(_) => panic!("missing local metadata must fail closed"),
        Err(err) => err,
    };
    assert!(
        err.to_string()
            .contains("Local repo not found for name wiki")
            || (err.to_string().contains("wiki") && err.to_string().contains("metadata missing")),
        "unexpected error: {err}"
    );
}

#[test]
fn open_remote_database_fails_closed_when_selector_metadata_is_missing() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))
        .expect("repo");
    let peer_id = PeerId::new("peer-a");
    let repo_id = uuid::Uuid::new_v4();
    repo.ensure_shadow_repo_info(
        &peer_id,
        &crate::ledger::RepoInfo {
            uuid: repo_id,
            name: "wiki".into(),
            url: Some("urn:wiki".into()),
        },
    )
    .expect("shadow repo info");
    let remote = repo
        .open_database(Some(&peer_id), "wiki")
        .expect("remote db");
    let txn = remote.db.begin_write().expect("write txn");
    txn.delete_table(REPO_METADATA).expect("delete metadata");
    txn.commit().expect("commit");

    let err = match repo.open_database(Some(&peer_id), "wiki") {
        Ok(_) => panic!("missing remote metadata must fail closed"),
        Err(err) => err,
    };
    assert!(
        err.to_string().contains("wiki")
            && err.to_string().contains("peer-a")
            && (err.to_string().contains("metadata missing")
                || err.to_string().contains("scanning catalog")),
        "unexpected error: {err}"
    );
}
