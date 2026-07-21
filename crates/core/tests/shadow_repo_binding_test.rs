use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_url(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        "urn:test:notes",
    )
    .expect("init repo");
    (dir, repo)
}

#[test]
fn ensure_shadow_repo_binding_keeps_shadow_repo_non_switchable_without_local_metadata_guess() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let info = repo
        .get_repo_info()
        .expect("read local repo info")
        .expect("local repo info exists");

    repo.ensure_shadow_repo_binding(&peer_id, info.uuid)
        .expect("ensure shadow binding");

    assert!(
        repo.list_repos(Some(&peer_id))
            .expect("list named remote repos")
            .is_empty()
    );
    assert_eq!(
        repo.find_remote_repo_selector_by_id(&peer_id, info.uuid)
            .expect("resolve shadow selector"),
        None
    );
    assert_eq!(
        repo.list_shadows_on_disk().expect("list shadows"),
        vec![peer_id.clone()]
    );
    assert!(
        repo.list_switchable_shadows_on_disk()
            .expect("list switchable shadows")
            .is_empty()
    );
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );
}

#[test]
fn list_shadows_on_disk_ignores_hidden_dirs() {
    let (_dir, repo) = new_repo();
    std::fs::create_dir_all(repo.remotes_dir().join(".invalid")).expect("hidden dir");
    let peer = PeerId::new("peer-visible");
    repo.ensure_shadow_repo_info(
        &peer,
        &deve_core::ledger::RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "visible".into(),
            url: Some("urn:test:visible".into()),
        },
    )
    .expect("seed visible shadow");

    assert_eq!(
        repo.list_shadows_on_disk().expect("list shadows"),
        vec![peer]
    );
}
