use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use tempfile::TempDir;

mod common;

#[test]
fn runtime_remote_listing_fails_closed_on_legacy_shadow_without_metadata() {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init repo");
    let info = common::add_initialized_local_repo(&repo, 10, "urn:test:wiki")
        .expect("init local companion");
    let peer_id = PeerId::new("peer-remote");

    common::seed_shadow_without_metadata_row(&repo, &peer_id, info.uuid);
    let err = repo
        .list_repos(Some(&peer_id))
        .expect_err("runtime listing must fail closed");
    assert!(
        err.to_string()
            .contains(format!("Broken shadow repo {} for peer {}", info.uuid, peer_id).as_str())
    );
}

#[test]
fn runtime_remote_selector_recovery_fails_closed_on_legacy_shadow_without_metadata() {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init repo");
    let info = common::add_initialized_local_repo(&repo, 10, "urn:test:wiki")
        .expect("init local companion");
    let peer_id = PeerId::new("peer-remote");

    common::seed_shadow_without_metadata_row(&repo, &peer_id, info.uuid);
    let err = repo
        .find_remote_repo_selector_by_id(&peer_id, info.uuid)
        .expect_err("runtime selector recovery must fail closed");
    assert!(
        err.to_string()
            .contains(format!("Broken shadow repo {} for peer {}", info.uuid, peer_id).as_str())
    );
}

#[test]
fn runtime_remote_repo_info_lookup_fails_closed_on_legacy_shadow_without_metadata() {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init repo");
    let info = common::add_initialized_local_repo(&repo, 10, "urn:test:wiki")
        .expect("init local companion");
    let peer_id = PeerId::new("peer-remote");

    common::seed_shadow_without_metadata_row(&repo, &peer_id, info.uuid);
    let err = repo
        .get_repo_info_for(Some(&peer_id), Some(&info.uuid.to_string()))
        .expect_err("runtime repo info lookup must fail closed");
    assert!(
        err.to_string()
            .contains(format!("Broken shadow repo {} for peer {}", info.uuid, peer_id).as_str())
    );
}
