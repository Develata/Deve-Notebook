use crate::ledger::{RepoInfo, RepoManager};
use tempfile::TempDir;

#[test]
fn local_repo_info_lookup_without_repair_preserves_unrepaired_metadata() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki_info =
        crate::test_support::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db");
    crate::test_support::write_repo_metadata(
        wiki_db.db.as_ref(),
        &RepoInfo {
            uuid: wiki_info.uuid,
            name: "alias".into(),
            url: Some("urn:alias".into()),
        },
    )
    .expect("write metadata");

    let looked_up = main
        .repo_scope_runtime()
        .get_local_repo_info_by_id_without_repair(wiki_info.uuid)
        .expect("lookup")
        .expect("present");
    assert_eq!(looked_up.name, "alias");
    assert_eq!(looked_up.url.as_deref(), Some("urn:alias"));
}

#[test]
fn local_repo_info_lookup_without_repair_fails_closed_on_broken_main_metadata() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let main_info = main.get_repo_info().expect("main info").expect("present");
    let main_db = main.open_database(None, "main").expect("main db");

    crate::test_support::poison_repo_metadata_invalid_bincode(main_db.db.as_ref())
        .expect("poison metadata");

    let err = main
        .repo_scope_runtime()
        .get_local_repo_info_by_id_without_repair(main_info.uuid)
        .expect_err("broken main metadata must fail closed");
    assert!(
        err.to_string()
            .contains("Broken local repo main while resolving UUID")
    );
}

#[test]
fn local_repo_name_by_url_fails_closed_on_missing_main_metadata() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let main_db = main.open_database(None, "main").expect("main db");

    crate::test_support::delete_repo_metadata(main_db.db.as_ref()).expect("delete metadata");

    let err = main
        .find_local_repo_name_by_url("urn:main")
        .expect_err("missing main metadata must fail closed");
    assert!(err.to_string().contains("repository metadata missing"));
}

#[test]
fn local_repo_name_by_url_fails_closed_on_duplicate_url_matches() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    crate::test_support::create_initialized_local_repo(&ledger_dir, "wiki", "urn:test");
    let mirror_info =
        crate::test_support::create_initialized_local_repo(&ledger_dir, "mirror", "urn:mirror");
    let mirror_db = main.open_database(None, "mirror").expect("mirror db").db;
    crate::test_support::write_repo_metadata(
        mirror_db.as_ref(),
        &RepoInfo {
            uuid: mirror_info.uuid,
            name: "mirror".into(),
            url: Some("urn:test".into()),
        },
    )
    .expect("write metadata");

    let err = main
        .find_local_repo_name_by_url("urn:test")
        .expect_err("duplicate local URL owners must fail closed");
    assert!(
        err.to_string()
            .contains("duplicate local repository URL urn:test")
    );
}
