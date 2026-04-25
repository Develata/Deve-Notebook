use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use tempfile::TempDir;

mod common;

#[test]
fn list_shadows_fails_closed_on_duplicate_shadow_uuids() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-dup");
    let info = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "notes".into(),
        url: Some("urn:test:notes".into()),
    };

    common::seed_shadow_repo_info(&repo, &peer_id, "notes", &info);
    common::seed_shadow_repo_info(&repo, &peer_id, "notes-1", &info);

    let err = repo
        .list_shadows_on_disk()
        .expect_err("duplicate uuid peer must fail shadow listing");
    assert!(
        err.to_string()
            .contains("duplicate remote repository UUIDs")
    );
}
