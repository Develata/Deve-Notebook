use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
use tempfile::TempDir;

fn write_info(db: &redb::Database, info: &RepoInfo) {
    let txn = db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(&0, bincode::serialize(info).expect("serialize").as_slice())
        .expect("write metadata");
    txn.commit().expect("commit");
}

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
    let wiki = RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki")).expect("wiki");
    let vault_root = dir.path().join("vault");
    std::fs::create_dir_all(vault_root.join("wiki")).expect("old root");
    std::fs::create_dir_all(vault_root.join("notes")).expect("new root");
    main.set_vault_root_checked(&vault_root)
        .expect("mount vault");

    let wiki_db = wiki.open_database(None, "wiki").expect("wiki db").db;
    let info = wiki.get_repo_info().expect("wiki info").expect("present");
    write_info(
        wiki_db.as_ref(),
        &RepoInfo {
            uuid: info.uuid,
            name: "notes".into(),
            url: info.url.clone(),
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
