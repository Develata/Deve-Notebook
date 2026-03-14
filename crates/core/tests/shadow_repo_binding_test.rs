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
fn ensure_shadow_repo_binding_keeps_shadow_repo_uuid_scoped_without_local_metadata_guess() {
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
    assert_eq!(listed, vec![info.uuid.to_string()]);
    let selector = repo
        .find_remote_repo_selector_by_id(&peer_id, info.uuid)
        .expect("resolve shadow selector")
        .expect("shadow selector");
    assert_eq!(selector, info.uuid.to_string());
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", selector))
            .exists()
    );
}

#[test]
fn list_shadows_on_disk_ignores_hidden_dirs() {
    let (_dir, repo) = new_repo();
    std::fs::create_dir_all(repo.remotes_dir().join(".invalid")).expect("hidden dir");
    let peer = PeerId::new("peer-visible");
    repo.ensure_shadow_db(&peer, &uuid::Uuid::new_v4())
        .expect("seed visible shadow");

    assert_eq!(
        repo.list_shadows_on_disk().expect("list shadows"),
        vec![peer]
    );
}
