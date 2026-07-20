use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use tempfile::TempDir;

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
