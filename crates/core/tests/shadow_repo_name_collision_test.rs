use deve_core::ledger::schema::REPO_METADATA;
use deve_core::ledger::{RepoInfo, RepoManager};
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
fn remote_repo_selector_by_name_fails_closed_on_name_collision() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    seed_shadow_file(
        &repo,
        &peer_id,
        "shadow-a",
        &RepoInfo {
            uuid: Uuid::new_v4(),
            name: "wiki/raw".into(),
            url: Some("urn:test:wiki-a".into()),
        },
    );
    seed_shadow_file(
        &repo,
        &peer_id,
        "shadow-b",
        &RepoInfo {
            uuid: Uuid::new_v4(),
            name: "wiki/raw".into(),
            url: Some("urn:test:wiki-b".into()),
        },
    );

    let err = repo
        .find_remote_repo_selector(&peer_id, "wiki/raw")
        .expect_err("duplicate display name must fail closed");
    assert!(
        err.to_string()
            .contains("ambiguous remote repository selector")
    );
}
