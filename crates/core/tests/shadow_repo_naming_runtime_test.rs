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

fn read_repo_info(db: &redb::Database) -> Option<RepoInfo> {
    let read = db.begin_read().expect("read txn");
    let table = read.open_table(REPO_METADATA).expect("repo metadata");
    let raw = table.get(&0).expect("read metadata")?;
    Some(bincode::deserialize(raw.value()).expect("deserialize repo info"))
}

#[test]
fn runtime_remote_repo_listing_does_not_repair_legacy_remote_filename() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let legacy_path = peer_dir.join("legacy.redb");
    {
        let db = Database::create(&legacy_path).expect("legacy db");
        let txn = db.begin_write().expect("write txn");
        txn.open_table(REPO_METADATA)
            .expect("repo metadata")
            .insert(
                &0,
                bincode::serialize(&RepoInfo {
                    uuid: repo_id,
                    name: "wiki".into(),
                    url: Some("urn:test:wiki".into()),
                })
                .expect("serialize")
                .as_slice(),
            )
            .expect("write metadata");
        txn.commit().expect("commit");
    }

    assert_eq!(
        repo.list_repos(Some(&peer_id)).expect("list remote repos"),
        vec!["wiki".to_string()]
    );
    assert!(!peer_dir.join("wiki.redb").exists());
    assert!(peer_dir.join("legacy.redb").exists());
}

#[test]
fn explicit_remote_catalog_repair_repairs_legacy_remote_filename() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let legacy_path = peer_dir.join("legacy.redb");
    {
        let db = Database::create(&legacy_path).expect("legacy db");
        let txn = db.begin_write().expect("write txn");
        txn.open_table(REPO_METADATA)
            .expect("repo metadata")
            .insert(
                &0,
                bincode::serialize(&RepoInfo {
                    uuid: repo_id,
                    name: "wiki".into(),
                    url: Some("urn:test:wiki".into()),
                })
                .expect("serialize")
                .as_slice(),
            )
            .expect("write metadata");
        txn.commit().expect("commit");
    }

    repo.repair_remote_repo_catalogs()
        .expect("repair remote catalogs");
    assert!(peer_dir.join("wiki.redb").exists());
    assert!(!peer_dir.join("legacy.redb").exists());
}

#[test]
fn runtime_remote_repo_listing_keeps_legacy_uuid_shadow_metadata_absent() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();

    repo.ensure_shadow_db(&peer_id, &repo_id)
        .expect("create legacy uuid shadow");

    assert_eq!(
        repo.list_repos(Some(&peer_id)).expect("list remote repos"),
        vec![repo_id.to_string()]
    );
    let handle = repo
        .open_database(Some(&peer_id), &repo_id.to_string())
        .expect("open legacy uuid shadow");
    assert!(read_repo_info(handle.db.as_ref()).is_none());
    assert_eq!(
        repo.find_remote_repo_selector_by_id(&peer_id, repo_id)
            .expect("resolve remote selector"),
        Some(repo_id.to_string())
    );
    assert!(read_repo_info(handle.db.as_ref()).is_none());
}
