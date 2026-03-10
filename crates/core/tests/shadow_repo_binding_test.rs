use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use tempfile::TempDir;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(
        dir.path().join("ledger"),
        10,
        Some("notes"),
        Some("urn:test:notes"),
    )
    .expect("init repo");
    (dir, repo)
}

#[test]
fn ensure_shadow_repo_binding_copies_local_repo_metadata() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let info = repo
        .get_repo_info()
        .expect("read local repo info")
        .expect("local repo info exists");

    repo.ensure_shadow_repo_binding(&peer_id, info.uuid)
        .expect("ensure shadow binding");

    let listed = repo
        .list_repos(Some(&peer_id))
        .expect("list named remote repos");
    assert_eq!(listed, vec!["notes".to_string()]);
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join("notes.redb")
            .exists()
    );
}
