use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
use tempfile::TempDir;

mod common;

#[test]
fn local_repo_listing_fails_closed_on_hidden_non_redb_entry() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let local_dir = ledger_dir.join("local");
    std::fs::write(local_dir.join(".stale"), b"local-junk").expect("hidden junk");

    let list_err = repo
        .list_repos(None)
        .expect_err("hidden non-redb local entry must fail listing");
    assert!(list_err.to_string().contains("unexpected non-redb entry"));

    let exec_err = repo
        .list_local_repo_names_for_execution()
        .expect_err("hidden non-redb local entry must fail execution listing");
    assert!(exec_err.to_string().contains("unexpected non-redb entry"));
}

#[test]
fn repair_local_repo_catalog_fails_closed_on_workspace_root_conflict() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let mut main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki_info = common::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");
    let vault_root = dir.path().join("vault");
    std::fs::create_dir_all(vault_root.join("wiki")).expect("old root");
    std::fs::create_dir_all(vault_root.join("notes")).expect("new root");
    main.set_vault_root_checked(&vault_root)
        .expect("mount vault");

    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;
    common::write_repo_metadata(
        wiki_db.as_ref(),
        &RepoInfo {
            uuid: wiki_info.uuid,
            name: "notes".into(),
            url: wiki_info.url.clone(),
        },
    );

    let err = main
        .repair_local_repo_catalog()
        .expect_err("workspace root conflict must fail closed");
    assert!(
        err.to_string().contains("current workspace root")
            && err.to_string().contains("already exists")
    );
}
