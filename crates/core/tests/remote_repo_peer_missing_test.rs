use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use tempfile::TempDir;

#[test]
fn missing_shadow_peer_dir_fails_closed_in_listing_and_selector_recovery() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-missing");
    let info = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "notes".into(),
        url: Some("urn:test:notes".into()),
    };

    repo.ensure_shadow_repo_info(&peer_id, &info)
        .expect("seed shadow");
    std::fs::remove_dir_all(repo.remotes_dir().join(peer_id.to_filename()))
        .expect("remove peer dir");

    let list_err = repo
        .list_repos(Some(&peer_id))
        .expect_err("missing peer dir must fail listing");
    assert!(list_err.to_string().contains(
        "Broken shadow peer peer-missing while pure scanning catalog: directory missing"
    ));

    let selector_err = repo
        .find_remote_repo_selector(&peer_id, "notes")
        .expect_err("missing peer dir must fail selector recovery");
    assert!(selector_err.to_string().contains(
        "Broken shadow peer peer-missing while pure scanning catalog: directory missing"
    ));
}
