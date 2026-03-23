use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::schema::REPO_METADATA;
use deve_core::models::PeerId;
use redb::Database;
use tempfile::TempDir;
use uuid::Uuid;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    (dir, repo)
}

fn seed_shadow_file(repo: &RepoManager, peer_id: &PeerId, stem: &str, info: &RepoInfo) {
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let path = peer_dir.join(format!("{}.redb", stem));
    let db = Database::create(&path).expect("shadow db");
    let write = db.begin_write().expect("write txn");
    write
        .open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(
            &0,
            bincode::serialize(info)
                .expect("serialize repo info")
                .as_slice(),
        )
        .expect("write repo info");
    write.commit().expect("commit repo info");
}

#[test]
fn duplicate_shadow_uuid_fails_closed_in_listing_and_selector_recovery() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let info = RepoInfo {
        uuid: repo_id,
        name: "wiki".into(),
        url: Some("urn:test:wiki".into()),
    };
    seed_shadow_file(&repo, &peer_id, "wiki", &info);
    seed_shadow_file(&repo, &peer_id, "wiki-1", &info);

    let list_err = repo
        .list_repos(Some(&peer_id))
        .expect_err("duplicate uuid must fail closed during listing");
    assert!(
        list_err
            .to_string()
            .contains("duplicate remote repository UUIDs")
    );
    let selector_err = repo
        .find_remote_repo_selector_by_id(&peer_id, repo_id)
        .expect_err("duplicate uuid selector recovery must fail closed");
    assert!(
        selector_err
            .to_string()
            .contains("duplicate remote repository UUIDs")
    );
}

#[test]
fn broken_shadow_file_fails_closed_even_if_metadata_was_previously_loaded() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let info = RepoInfo {
        uuid: Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-a".into()),
    };
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    let path = peer_dir.join("wiki.redb");

    repo.ensure_shadow_repo_info(&peer_id, &info)
        .expect("prepare shadow repo");
    std::fs::remove_file(&path).expect("remove live shadow db");
    std::fs::write(&path, b"not-a-redb").expect("replace shadow db with broken bytes");

    let list_err = repo
        .list_repos(Some(&peer_id))
        .expect_err("broken shadow listing must fail closed");
    let list_detail = list_err.to_string();
    let expected_uuid = info.uuid.to_string();
    assert!(list_detail.contains("Broken shadow repo"));
    assert!(list_detail.contains("for peer peer-remote"));
    assert!(list_detail.contains("wiki") || list_detail.contains(&expected_uuid));
    let selector_err = repo
        .find_remote_repo_selector_by_id(&peer_id, info.uuid)
        .expect_err("broken shadow selector must fail closed");
    let selector_detail = selector_err.to_string();
    assert!(selector_detail.contains("Broken shadow repo"));
    assert!(selector_detail.contains("for peer peer-remote"));
    assert!(selector_detail.contains("wiki") || selector_detail.contains(&expected_uuid));
}
