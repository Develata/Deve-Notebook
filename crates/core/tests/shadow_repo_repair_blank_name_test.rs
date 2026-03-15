use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::schema::REPO_METADATA;
use deve_core::models::PeerId;
use redb::Database;
use tempfile::TempDir;
use uuid::Uuid;

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
fn remote_catalog_repair_uses_uuid_selector_for_blank_non_uuid_shadow_name() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-remote");
    let info = RepoInfo {
        uuid: Uuid::new_v4(),
        name: String::new(),
        url: Some("urn:test:shadow".into()),
    };
    seed_shadow_file(&repo, &peer_id, "legacy-shadow", &info);

    repo.repair_remote_repo_catalogs()
        .expect("repair remote catalogs");

    assert_eq!(
        repo.list_repos(Some(&peer_id)).expect("list repaired shadows"),
        vec![info.uuid.to_string()]
    );
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );
}
