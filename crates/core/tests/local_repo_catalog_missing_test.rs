use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use tempfile::TempDir;

#[test]
fn local_repo_listing_fails_closed_when_local_catalog_dir_is_missing() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki = RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki")).expect("wiki");
    let wiki_id = wiki
        .get_repo_info()
        .expect("wiki info")
        .expect("present")
        .uuid;

    std::fs::remove_dir_all(ledger_dir.join("local")).expect("remove local catalog dir");

    let list_err = repo
        .list_repos(None)
        .expect_err("missing local repo dir must fail listing");
    assert!(
        list_err
            .to_string()
            .contains("local repo directory missing")
    );

    let exec_err = repo
        .list_local_repo_names_for_execution()
        .expect_err("missing local repo dir must fail execution listing");
    assert!(
        exec_err
            .to_string()
            .contains("local repo directory missing")
    );

    let lookup_err = repo
        .find_local_repo_name_by_id(wiki_id)
        .expect_err("missing local repo dir must fail UUID lookup");
    assert!(
        lookup_err
            .to_string()
            .contains("local repo directory missing")
    );
}
