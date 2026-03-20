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

    let repos = repo.list_repos(Some(&peer_id)).expect("list remote repos");
    assert_eq!(repos, vec!["legacy".to_string()]);
    let handle = repo
        .open_database(Some(&peer_id), &repos[0])
        .expect("open remote shadow by exact selector");
    assert_eq!(handle.repo_name, "legacy");
    assert_eq!(handle.repo_id, Some(repo_id));
    assert!(!peer_dir.join("wiki.redb").exists());
    assert!(peer_dir.join("legacy.redb").exists());
}

#[test]
fn runtime_remote_display_name_selector_fails_closed_when_stem_drifted() {
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
        repo.find_remote_repo_selector(&peer_id, "wiki")
            .expect("resolve legacy display name"),
        None
    );
    let err = match repo.open_database(Some(&peer_id), "wiki") {
        Ok(_) => panic!("display-only selector must fail closed"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("Repository not found: wiki"));
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
fn runtime_remote_repo_listing_fails_closed_on_legacy_uuid_shadow_without_metadata() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();

    repo.ensure_shadow_db(&peer_id, &repo_id)
        .expect("create legacy uuid shadow");

    let list_err = repo
        .list_repos(Some(&peer_id))
        .expect_err("runtime listing must fail closed on metadata-less uuid shadow");
    assert!(
        list_err
            .to_string()
            .contains(format!("Broken shadow repo {} for peer {}", repo_id, peer_id).as_str())
    );
    let open_err = match repo.open_database(Some(&peer_id), &repo_id.to_string()) {
        Ok(_) => panic!("runtime open must fail closed on metadata-less uuid shadow"),
        Err(err) => err,
    };
    assert!(
        open_err
            .to_string()
            .contains(format!("Broken shadow repo {} for peer {}", repo_id, peer_id).as_str())
    );
}
