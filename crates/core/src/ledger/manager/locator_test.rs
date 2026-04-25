use super::RepoManager;
use crate::ledger::RepoInfo;
use tempfile::TempDir;

#[test]
fn local_repo_id_lookup_without_repair_uses_current_on_disk_metadata() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki_info =
        crate::test_support::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db");
    crate::test_support::write_repo_metadata(
        wiki_db.db.as_ref(),
        &RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:wiki".into()),
        },
    )
    .expect("write metadata");

    assert_eq!(
        main.find_local_repo_name_by_id_without_repair(wiki_info.uuid)
            .expect("lookup without repair"),
        None
    );
}

#[test]
fn local_repo_id_lookup_without_repair_fails_closed_on_missing_secondary_metadata() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki_info =
        crate::test_support::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;

    crate::test_support::delete_repo_metadata(wiki_db.as_ref()).expect("delete metadata");

    let err = main
        .find_local_repo_name_by_id_without_repair(wiki_info.uuid)
        .expect_err("missing secondary metadata must fail closed");
    assert!(err.to_string().contains("repository metadata missing"));
}
