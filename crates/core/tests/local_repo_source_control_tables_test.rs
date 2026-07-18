use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::DocId;
use deve_core::source_control::staging;
use tempfile::TempDir;

mod common;

fn seed_legacy_local_repo(path: &std::path::Path, info: &RepoInfo) {
    common::seed_local_repo_missing_source_control_tables(path, info);
}

#[test]
fn local_catalog_fails_closed_on_missing_secondary_source_control_tables_until_repair() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let legacy_id = uuid::Uuid::new_v4();
    let legacy_path = ledger_dir.join("local").join(format!("{legacy_id}.redb"));
    seed_legacy_local_repo(
        &legacy_path,
        &RepoInfo {
            uuid: legacy_id,
            name: "legacy".into(),
            url: Some("urn:legacy".into()),
        },
    );

    let list_err = repo
        .list_repos(None)
        .expect_err("missing source control tables must fail local listing");
    assert!(list_err.to_string().contains("source control tables"));
    let pending_err = repo
        .list_pending_fs_in_local_repo("legacy")
        .expect_err("missing source control tables must fail local pending listing");
    let pending_detail = pending_err.to_string();
    assert!(
        pending_detail.contains("source control tables"),
        "unexpected pending error: {pending_detail}"
    );

    repo.repair_local_repo_catalog()
        .expect("repair local repo catalog");
    assert_eq!(
        repo.list_repos(None)
            .expect("list local repos after repair"),
        vec!["legacy".to_string(), "main".to_string()]
    );
    assert!(
        repo.list_pending_fs_in_local_repo("legacy")
            .expect("list pending after repair")
            .is_empty()
    );
}

#[test]
fn main_local_repo_fails_closed_on_missing_source_control_tables_until_repair() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");

    let handle = repo.open_database(None, "main").expect("open main db");
    let write = handle.db.begin_write().expect("write txn");
    let _ = write
        .delete_table(staging::STAGED_TABLE)
        .expect("delete staged table");
    write.commit().expect("commit table delete");

    let staged_err = repo
        .list_staged()
        .expect_err("missing source control tables must fail staged listing");
    assert!(
        staged_err
            .to_string()
            .contains(&format!("Broken local repo {}", repo.local_repo_name()))
    );
    assert!(staged_err.to_string().contains("source control tables"));

    let commits_err = repo
        .list_commits(10)
        .expect_err("missing source control tables must fail commit listing");
    assert!(
        commits_err
            .to_string()
            .contains(&format!("Broken local repo {}", repo.local_repo_name()))
    );
    assert!(commits_err.to_string().contains("source control tables"));

    let content_err = repo
        .get_committed_content(DocId::new())
        .expect_err("missing source control tables must fail committed content lookup");
    assert!(
        content_err
            .to_string()
            .contains(&format!("Broken local repo {}", repo.local_repo_name()))
    );
    assert!(content_err.to_string().contains("source control tables"));

    repo.repair_local_repo_catalog()
        .expect("repair local repo catalog");
    assert!(
        repo.list_staged()
            .expect("list staged after repair")
            .is_empty()
    );
    assert!(
        repo.list_commits(10)
            .expect("list commits after repair")
            .is_empty()
    );
}
