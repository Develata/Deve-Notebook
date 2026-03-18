use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::schema::REPO_METADATA;
use deve_core::models::PeerId;
use redb::Database;
use tempfile::TempDir;

fn seed_shadow_file(repo: &RepoManager, peer_id: &PeerId, stem: &str, info: &RepoInfo) {
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let db = Database::create(peer_dir.join(format!("{stem}.redb"))).expect("shadow db");
    let write = db.begin_write().expect("write txn");
    write
        .open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(
            &0,
            bincode::serialize(info).expect("serialize info").as_slice(),
        )
        .expect("write info");
    write.commit().expect("commit");
}

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

    seed_shadow_file(&repo, &peer_id, "notes", &info);
    seed_shadow_file(&repo, &peer_id, "notes-1", &info);

    let err = repo
        .list_shadows_on_disk()
        .expect_err("duplicate uuid peer must fail shadow listing");
    assert!(
        err.to_string()
            .contains("duplicate remote repository UUIDs")
    );
}
