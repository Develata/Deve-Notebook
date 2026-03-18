use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
use tempfile::TempDir;

fn seed_legacy_local_repo(path: &std::path::Path, info: &RepoInfo) {
    let db = redb::Database::create(path).expect("create legacy db");
    let write = db.begin_write().expect("write txn");
    write
        .open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(
            &0,
            bincode::serialize(info).expect("serialize info").as_slice(),
        )
        .expect("write metadata");
    write.commit().expect("commit");
}

#[test]
fn local_catalog_fails_closed_on_missing_secondary_source_control_tables_until_repair() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let legacy_path = ledger_dir.join("local").join("legacy.redb");
    seed_legacy_local_repo(
        &legacy_path,
        &RepoInfo {
            uuid: uuid::Uuid::new_v4(),
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
    assert!(pending_err.to_string().contains("source control tables"));

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
