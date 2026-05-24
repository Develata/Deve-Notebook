use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
use tempfile::TempDir;

mod common;

#[test]
fn repair_realigns_workspace_root_to_repaired_repo_name() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("projection-base");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let main_db = repo.open_database(None, "main").expect("main db").db;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("projection locator");

    std::fs::create_dir_all(projection_base.join("legacy/.notegit")).expect("legacy workspace");
    common::write_repo_metadata(
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

    repo.repair_local_repo_catalog()
        .expect("repair local catalog realigns workspace");

    assert!(projection_base.join("main/.notegit").exists());
    assert!(!projection_base.join("legacy").exists());
}

#[test]
fn runtime_catalog_refresh_does_not_realign_workspace_root() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("projection-base");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("projection locator");
    let main_db = repo.open_database(None, "main").expect("main db").db;

    common::write_repo_metadata(
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
    std::fs::create_dir_all(projection_base.join("legacy/.notegit")).expect("legacy workspace");
    std::fs::write(projection_base.join("legacy/note.md"), "hello").expect("write note");

    let err = repo
        .list_repos(None)
        .expect_err("runtime catalog refresh must fail closed on drift");
    assert!(
        err.to_string().contains("metadata name drifted to legacy"),
        "unexpected error: {err:#}"
    );
    assert_eq!(
        repo.get_repo_info()
            .expect("repo info")
            .expect("present")
            .name,
        "legacy"
    );
    assert!(projection_base.join("legacy/.notegit").exists());
    assert!(projection_base.join("legacy/note.md").exists());
    assert!(!projection_base.join("main").exists());
}
