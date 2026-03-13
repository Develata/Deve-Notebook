use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use tempfile::TempDir;

#[test]
fn list_shadows_ignores_empty_peer_dirs() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-empty");
    std::fs::create_dir_all(repo.remotes_dir().join(peer_id.to_filename())).expect("peer dir");

    assert!(
        repo.list_shadows_on_disk()
            .expect("list shadows")
            .is_empty()
    );
}
