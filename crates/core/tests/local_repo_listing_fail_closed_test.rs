use deve_core::ledger::listing::RepoListing;
use tempfile::TempDir;

mod common;

#[test]
fn local_repo_listing_fails_closed_on_missing_main_metadata() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, main_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes"), 8)
            .expect("main");
    let main_db = repo
        .lease_local_authority(main_id)
        .expect("main authority lease");

    common::delete_repo_metadata(main_db.db());

    let err = repo
        .list_repos(None)
        .expect_err("missing main metadata must fail closed");
    assert!(err.to_string().contains("repository metadata missing"));
}
