use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{REPO_METADATA, RepoManager};
use tempfile::TempDir;

#[test]
fn local_repo_listing_fails_closed_on_missing_main_metadata() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let main_db = repo.open_database(None, "main").expect("main db");

    let txn = main_db.db.begin_write().expect("write txn");
    txn.delete_table(REPO_METADATA).expect("delete metadata");
    txn.commit().expect("commit");

    let err = repo
        .list_repos(None)
        .expect_err("missing main metadata must fail closed");
    assert!(err.to_string().contains("repository metadata missing"));
}
