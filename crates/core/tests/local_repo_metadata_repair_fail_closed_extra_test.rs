use deve_core::ledger::listing::RepoListing;
use tempfile::TempDir;

mod common;

#[test]
fn local_repo_listing_ignores_hidden_nonmember_and_repair_reports_it() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, repo_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes"), 8)
            .expect("main");
    let local_dir = ledger_dir.join("local");
    std::fs::write(local_dir.join(".stale"), b"local-junk").expect("hidden junk");

    assert_eq!(
        repo.list_repos(None).expect("catalog listing"),
        vec![repo_id.to_string()]
    );
    assert_eq!(
        repo.list_local_repo_names_for_execution()
            .expect("execution listing"),
        vec![repo_id.to_string()]
    );
    let repair_err = repo
        .repair_local_repo_catalog()
        .expect_err("explicit repair must report hidden non-redb local entry");
    assert!(repair_err.to_string().contains("unexpected non-redb entry"));
}
