use crate::ledger::RepoInfo;
use tempfile::TempDir;

#[test]
fn local_repo_info_lookup_without_repair_rejects_unrepaired_metadata() {
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
    // Drift only the display metadata (name/url); keep the physical RepoId so the
    // without-repair lookup still resolves and returns the raw drifted metadata.
    crate::test_support::write_repo_metadata(
        wiki_db.db(),
        &RepoInfo {
            uuid: wiki_id,
            name: "alias".into(),
            url: Some("urn:alias".into()),
        },
    )
    .expect("write metadata");

    let error = main
        .repo_scope_runtime()
        .get_local_repo_info_by_id_without_repair(wiki_id)
        .expect_err("noncanonical machine name must fail closed");
    assert!(error.to_string().contains("metadata identity mismatch"));
}

#[test]
fn local_repo_info_lookup_without_repair_fails_closed_on_broken_main_metadata() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, main_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))
            .expect("main");
    let main_db = main
        .local_authority_lease_for_test(main_id)
        .expect("main db");

    crate::test_support::poison_repo_metadata_invalid_codec(main_db.db()).expect("poison metadata");

    let err = main
        .repo_scope_runtime()
        .get_local_repo_info_by_id_without_repair(main_id)
        .expect_err("broken main metadata must fail closed");
    assert!(err.to_string().contains("postcard deserialization failed"));
}

#[test]
fn local_repo_name_by_url_fails_closed_on_missing_main_metadata() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, main_id) = crate::test_support::init_cataloged_repo_with_url(
        &ledger_dir,
        &dir.path().join("notes"),
        "urn:main",
    )
    .expect("main");
    let main_db = main
        .local_authority_lease_for_test(main_id)
        .expect("main db");

    crate::test_support::delete_repo_metadata(main_db.db()).expect("delete metadata");

    let err = main
        .find_local_repo_name_by_url("urn:main")
        .expect_err("missing main metadata must fail closed");
    assert!(err.to_string().contains("repository metadata missing"));
}

#[test]
fn local_repo_name_by_url_fails_closed_on_duplicate_url_matches() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) = crate::test_support::init_cataloged_repo_with_url(
        &ledger_dir,
        &dir.path().join("main-notes"),
        "urn:main",
    )
    .expect("main");
    let _wiki_id = crate::test_support::add_cataloged_repo(
        &main,
        &ledger_dir,
        &dir.path().join("wiki-notes"),
        Some("urn:test"),
    )
    .expect("wiki");
    let mirror_id = crate::test_support::add_cataloged_repo(
        &main,
        &ledger_dir,
        &dir.path().join("mirror-notes"),
        Some("urn:mirror"),
    )
    .expect("mirror");
    let mirror_db = main
        .local_authority_lease_for_test(mirror_id)
        .expect("mirror db");
    // Drift the mirror URL to collide with wiki's, keeping the physical identity
    // canonical so catalog validation reaches the duplicate-URL check.
    crate::test_support::write_repo_metadata(
        mirror_db.db(),
        &RepoInfo {
            uuid: mirror_id,
            name: mirror_id.to_string(),
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
