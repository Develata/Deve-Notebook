use crate::ledger::RepoInfo;
use tempfile::TempDir;

#[test]
fn local_repo_id_lookup_without_repair_uses_current_on_disk_metadata() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("main-notes"))
            .expect("main");
    let (_wiki, wiki_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("wiki-notes"))
            .expect("wiki");
    let wiki_db = main
        .open_database(None, &wiki_id.to_string())
        .expect("wiki db");
    // Rewrite the on-disk metadata with a fresh RepoId, so a lookup for the
    // original id must observe current disk state and find nothing.
    let drifted = uuid::Uuid::new_v4();
    crate::test_support::write_repo_metadata(
        wiki_db.db.as_ref(),
        &RepoInfo {
            uuid: drifted,
            name: drifted.to_string(),
            url: Some("urn:wiki".into()),
        },
    )
    .expect("write metadata");

    assert_eq!(
        main.repo_scope_runtime()
            .find_local_repo_name_by_id_without_repair(wiki_id)
            .expect("lookup without repair"),
        None
    );
}

#[test]
fn local_repo_id_lookup_without_repair_fails_closed_on_missing_secondary_metadata() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("main-notes"))
            .expect("main");
    let (_wiki, wiki_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("wiki-notes"))
            .expect("wiki");
    let wiki_db = main
        .open_database(None, &wiki_id.to_string())
        .expect("wiki db")
        .db;

    crate::test_support::delete_repo_metadata(wiki_db.as_ref()).expect("delete metadata");

    let err = main
        .repo_scope_runtime()
        .find_local_repo_name_by_id_without_repair(wiki_id)
        .expect_err("missing secondary metadata must fail closed");
    assert!(err.to_string().contains("repository metadata missing"));
}
