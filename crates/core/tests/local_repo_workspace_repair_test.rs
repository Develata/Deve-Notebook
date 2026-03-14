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
fn repair_realigns_workspace_root_to_repaired_repo_name() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let vault_dir = dir.path().join("vault");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let main_db = repo.open_database(None, "main").expect("main db").db;

    std::fs::create_dir_all(vault_dir.join("legacy/.notegit")).expect("legacy workspace");
    std::fs::write(vault_dir.join("legacy/note.md"), "hello").expect("write note");
    write_info(
        main_db.as_ref(),
        &RepoInfo {
            uuid: repo
                .get_repo_info()
                .expect("main info")
                .expect("present")
                .uuid,
            name: "legacy".into(),
            url: Some("urn:main".into()),
        },
    );

    repo.set_vault_root(&vault_dir);

    assert!(vault_dir.join("main/.notegit").exists());
    assert!(vault_dir.join("main/note.md").exists());
    assert!(!vault_dir.join("legacy").exists());
}

#[test]
fn runtime_catalog_refresh_does_not_realign_workspace_root() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let vault_dir = dir.path().join("vault");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    repo.set_vault_root(&vault_dir);
    let main_db = repo.open_database(None, "main").expect("main db").db;

    write_info(
        main_db.as_ref(),
        &RepoInfo {
            uuid: repo
                .get_repo_info()
                .expect("main info")
                .expect("present")
                .uuid,
            name: "legacy".into(),
            url: Some("urn:main".into()),
        },
    );
    std::fs::create_dir_all(vault_dir.join("legacy/.notegit")).expect("legacy workspace");
    std::fs::write(vault_dir.join("legacy/note.md"), "hello").expect("write note");

    assert_eq!(
        repo.list_repos(None).expect("runtime repo listing"),
        vec!["main".to_string()]
    );
    assert_eq!(
        repo.get_repo_info()
            .expect("repaired metadata")
            .expect("present")
            .name,
        "main"
    );
    assert!(vault_dir.join("legacy/.notegit").exists());
    assert!(vault_dir.join("legacy/note.md").exists());
    assert!(!vault_dir.join("main").exists());
}
