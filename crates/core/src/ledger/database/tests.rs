use super::{RepoManager, cached_shadow_database};
use crate::models::PeerId;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
#[test]
fn cached_shadow_database_fails_closed_when_path_is_unstatable() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let blocked = dir.path().join("blocked");
    std::fs::create_dir_all(&blocked).expect("blocked dir");
    let original = std::fs::metadata(&blocked).expect("metadata").permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(&blocked, perms).expect("chmod 000");
    let path = blocked.join("notes.redb");

    let err = cached_shadow_database(&path).expect_err("unstatable path must fail closed");

    std::fs::set_permissions(&blocked, original).expect("restore perms");
    assert!(
        err.to_string().contains("Failed to stat database path")
            || err.to_string().contains("Permission denied")
    );
}

#[test]
fn cached_shadow_database_never_initializes_an_empty_existing_file() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.redb");
    std::fs::File::create(&path).expect("empty file");

    let error =
        cached_shadow_database(&path).expect_err("existing opener must not initialize a DB");

    assert!(!error.to_string().is_empty());
    assert_eq!(std::fs::metadata(&path).expect("metadata").len(), 0);
}

#[cfg(unix)]
#[test]
fn open_local_database_fails_closed_when_path_is_unstatable() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, _main_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("main-notes"))
            .expect("main");
    let wiki_id = crate::test_support::add_cataloged_repo(
        &repo,
        &ledger_dir,
        &dir.path().join("wiki-notes"),
        None,
    )
    .expect("wiki");
    let wiki_name = wiki_id.to_string();
    let local_dir = dir.path().join("ledger/local");
    let original = std::fs::metadata(&local_dir)
        .expect("metadata")
        .permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(&local_dir, perms).expect("chmod 000");

    let err = match repo.open_database(None, &wiki_name) {
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
    let (repo, _main_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("main-notes"))
            .expect("main");
    let wiki_id = crate::test_support::add_cataloged_repo(
        &repo,
        &ledger_dir,
        &dir.path().join("wiki-notes"),
        None,
    )
    .expect("wiki");
    let wiki_name = wiki_id.to_string();
    let wiki_db = repo
        .local_authority_lease_for_test(wiki_id)
        .expect("wiki db");
    crate::test_support::delete_repo_metadata(wiki_db.db()).expect("delete metadata");

    let err = match repo.open_database(None, &wiki_name) {
        Ok(_) => panic!("missing local metadata must fail closed"),
        Err(err) => err,
    };
    let message = err.to_string();
    assert!(
        message.contains(&wiki_name) && message.contains("metadata missing"),
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
    crate::test_support::delete_repo_metadata(remote.remote_db().expect("remote db"))
        .expect("delete metadata");

    let err = match repo.open_database(Some(&peer_id), "wiki") {
        Ok(_) => panic!("missing remote metadata must fail closed"),
        Err(err) => err,
    };
    assert!(
        err.to_string().contains(&repo_id.to_string())
            && err.to_string().contains("peer-a")
            && (err.to_string().contains("metadata missing")
                || err.to_string().contains("scanning catalog")),
        "unexpected error: {err}"
    );
}
