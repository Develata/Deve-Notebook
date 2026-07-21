use crate::ledger::RepoInfo;
use tempfile::TempDir;

#[test]
fn local_repo_id_lookup_without_repair_rejects_metadata_identity_drift() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("main-notes"))
            .expect("main");
    let wiki_id = crate::test_support::add_cataloged_repo(
        &main,
        &ledger_dir,
        &dir.path().join("wiki-notes"),
        None,
    )
    .expect("wiki");
    let wiki_db = main
        .local_authority_lease_for_test(wiki_id)
        .expect("wiki db");
    // Rewrite the on-disk metadata with a fresh RepoId, so a lookup for the
    // original id must observe current disk state and find nothing.
    let drifted = uuid::Uuid::new_v4();
    crate::test_support::write_repo_metadata(
        wiki_db.db(),
        &RepoInfo {
            uuid: drifted,
            name: drifted.to_string(),
            url: Some("urn:wiki".into()),
        },
    )
    .expect("write metadata");

    let error = main
        .repo_scope_runtime()
        .find_local_repo_name_by_id_without_repair(wiki_id)
        .expect_err("identity drift must fail closed");
    assert!(error.to_string().contains("metadata identity mismatch"));
}

#[test]
fn local_repo_id_lookup_without_repair_fails_closed_on_missing_secondary_metadata() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("main-notes"))
            .expect("main");
    let wiki_id = crate::test_support::add_cataloged_repo(
        &main,
        &ledger_dir,
        &dir.path().join("wiki-notes"),
        None,
    )
    .expect("wiki");
    let wiki_db = main
        .local_authority_lease_for_test(wiki_id)
        .expect("wiki db");

    crate::test_support::delete_repo_metadata(wiki_db.db()).expect("delete metadata");

    let err = main
        .repo_scope_runtime()
        .find_local_repo_name_by_id_without_repair(wiki_id)
        .expect_err("missing secondary metadata must fail closed");
    assert!(err.to_string().contains("repository metadata missing"));
}
