use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
use tempfile::TempDir;

mod common;

fn workspace_segment(name: &str, repo_id: uuid::Uuid) -> String {
    format!("{name}--{repo_id}")
}

#[test]
fn repair_realigns_workspace_root_to_repaired_repo_name() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("projection-base");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let main_db = repo.open_database(None, "main").expect("main db").db;
    let repo_id = repo
        .get_repo_info()
        .expect("main info")
        .expect("present")
        .uuid;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("projection locator");

    std::fs::create_dir_all(
        projection_base
            .join(workspace_segment("legacy", repo_id))
            .join(".notegit"),
    )
    .expect("legacy workspace");
    common::write_repo_metadata(
        main_db.as_ref(),
        &RepoInfo {
            uuid: repo_id,
            name: "legacy".into(),
            url: Some("urn:main".into()),
        },
    );

    repo.repair_local_repo_catalog()
        .expect("repair local catalog realigns workspace");

    assert!(
        projection_base
            .join(workspace_segment("main", repo_id))
            .join(".notegit")
            .exists()
    );
    deve_core::utils::notegit::validate_repo_identity_marker(
        &projection_base.join(workspace_segment("main", repo_id)),
        repo_id,
    )
    .expect("identity marker");
    assert!(
        !projection_base
            .join(workspace_segment("legacy", repo_id))
            .exists()
    );
}

#[test]
fn repair_migrates_legacy_repo_name_workspace_root_to_repo_id_segment() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("projection-base");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let repo_id = repo
        .get_repo_info()
        .expect("main info")
        .expect("present")
        .uuid;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("projection locator");

    std::fs::create_dir_all(projection_base.join("main/.notegit")).expect("legacy workspace");

    let err = repo
        .local_repo_workspace_root("main")
        .expect_err("legacy workspace root must fail closed before repair");
    assert!(
        err.to_string().contains("legacy repo-name path"),
        "unexpected error: {err:#}"
    );

    repo.repair_local_repo_catalog()
        .expect("repair migrates legacy workspace root");

    let root = projection_base.join(workspace_segment("main", repo_id));
    assert!(root.join(".notegit").exists());
    deve_core::utils::notegit::validate_repo_identity_marker(&root, repo_id)
        .expect("identity marker");
    assert!(!projection_base.join("main").exists());
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
    let repo_id = repo
        .get_repo_info()
        .expect("main info")
        .expect("present")
        .uuid;

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
    let legacy_root = projection_base.join(workspace_segment("legacy", repo_id));
    std::fs::create_dir_all(legacy_root.join(".notegit")).expect("legacy workspace");
    std::fs::write(legacy_root.join("note.md"), "hello").expect("write note");

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
    assert!(legacy_root.join(".notegit").exists());
    assert!(legacy_root.join("note.md").exists());
    assert!(
        !projection_base
            .join(workspace_segment("main", repo_id))
            .exists()
    );
}
