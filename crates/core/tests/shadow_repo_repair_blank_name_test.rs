use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use tempfile::TempDir;
use uuid::Uuid;

mod common;

#[test]
fn remote_catalog_repair_uses_uuid_selector_for_blank_non_uuid_shadow_name() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-remote");
    let info = RepoInfo {
        uuid: Uuid::new_v4(),
        name: String::new(),
        url: Some("urn:test:shadow".into()),
    };
    common::seed_shadow_repo_info(&repo, &peer_id, "legacy-shadow", &info);

    repo.repair_remote_repo_catalogs()
        .expect("repair remote catalogs");

    assert_eq!(
        repo.list_repos(Some(&peer_id))
            .expect("list repaired shadows"),
        vec![info.uuid.to_string()]
    );
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );
}
