use super::RepoManager;
use crate::ledger::{REPO_METADATA, RepoInfo};
use tempfile::TempDir;

fn write_info(db: &redb::Database, info: &RepoInfo) {
    let txn = db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(&0, bincode::serialize(info).expect("serialize").as_slice())
        .expect("write metadata");
    txn.commit().expect("commit");
}

fn create_secondary_repo(ledger_dir: &std::path::Path, name: &str, url: &str) -> RepoInfo {
    let repo = RepoManager::init(ledger_dir, 8, Some(name), Some(url)).expect("secondary repo");
    repo.get_repo_info()
        .expect("secondary info")
        .expect("secondary metadata")
}

#[test]
fn local_repo_id_lookup_without_repair_uses_current_on_disk_metadata() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki_info = create_secondary_repo(&ledger_dir, "wiki", "urn:wiki");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db");
    write_info(
        wiki_db.db.as_ref(),
        &RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:wiki".into()),
        },
    );

    assert_eq!(
        main.find_local_repo_name_by_id_without_repair(wiki_info.uuid)
            .expect("lookup without repair"),
        None
    );
}

#[test]
fn local_repo_id_lookup_without_repair_fails_closed_on_missing_secondary_metadata() {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki_info = create_secondary_repo(&ledger_dir, "wiki", "urn:wiki");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;

    let txn = wiki_db.begin_write().expect("write txn");
    txn.delete_table(REPO_METADATA).expect("delete metadata");
    txn.commit().expect("commit");

    let err = main
        .find_local_repo_name_by_id_without_repair(wiki_info.uuid)
        .expect_err("missing secondary metadata must fail closed");
    assert!(err.to_string().contains("repository metadata missing"));
}
