use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::schema::REPO_METADATA;
use deve_core::models::PeerId;
use tempfile::TempDir;

fn read_repo_info(db: &redb::Database) -> Option<deve_core::ledger::RepoInfo> {
    let read = db.begin_read().expect("read txn");
    let table = read.open_table(REPO_METADATA).expect("repo metadata");
    let raw = table.get(&0).expect("read metadata")?;
    Some(bincode::deserialize(raw.value()).expect("deserialize repo info"))
}

#[test]
fn remote_catalog_keeps_legacy_uuid_shadow_as_uuid_selector_without_remote_metadata() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let local = RepoManager::init(
        dir.path().join("ledger"),
        10,
        Some("wiki"),
        Some("urn:test:wiki"),
    )
    .expect("init local companion");
    let info = local
        .get_repo_info()
        .expect("local repo info")
        .expect("local repo exists");
    let peer_id = PeerId::new("peer-remote");

    repo.ensure_shadow_db(&peer_id, &info.uuid)
        .expect("create legacy uuid shadow");
    repo.repair_remote_repo_catalogs()
        .expect("repair remote catalogs explicitly");
    assert_eq!(
        repo.list_repos(Some(&peer_id))
            .expect("list repaired shadows"),
        vec![info.uuid.to_string()]
    );

    let handle = repo
        .open_database(Some(&peer_id), &info.uuid.to_string())
        .expect("open repaired shadow");
    let repaired = read_repo_info(&handle.db).expect("repo info written back");
    assert_eq!(repaired.name, info.uuid.to_string());
    assert_eq!(repaired.uuid, info.uuid);
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );
}

#[test]
fn init_keeps_uuid_shadow_path_without_remote_metadata() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let local = RepoManager::init(&ledger_dir, 10, Some("wiki"), Some("urn:test:wiki"))
        .expect("init local companion");
    let info = local
        .get_repo_info()
        .expect("local repo info")
        .expect("local repo exists");
    let peer_id = PeerId::new("peer-remote");

    repo.ensure_shadow_db(&peer_id, &info.uuid)
        .expect("create legacy uuid shadow");
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );

    let repaired = RepoManager::init(&ledger_dir, 10, None, None).expect("re-init repo");
    assert_eq!(
        repaired
            .list_repos(Some(&peer_id))
            .expect("list repaired shadows"),
        vec![info.uuid.to_string()]
    );
    assert!(
        repaired
            .remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );
}

#[test]
fn init_fails_closed_on_broken_shadow_catalogs() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-bad");
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("create peer dir");
    std::fs::write(peer_dir.join("broken.redb"), b"not-a-redb").expect("write broken shadow");

    let err = RepoManager::init(&ledger_dir, 10, None, None)
        .err()
        .expect("broken shadow peer must fail init");
    let detail = format!("{err:#}");
    assert!(detail.contains("Failed to repair remote repo catalogs during init"));
    assert!(detail.contains("Broken shadow peer peer-bad"));
}

#[test]
fn local_catalog_repair_does_not_make_legacy_shadow_runtime_readable() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 10, Some("main"), Some("urn:main")).expect("main");
    let wiki = RepoManager::init(&ledger_dir, 10, Some("wiki"), Some("urn:wiki")).expect("wiki");
    let peer_id = PeerId::new("peer-remote");
    let wiki_info = wiki.get_repo_info().expect("wiki info").expect("present");

    main.ensure_shadow_db(&peer_id, &wiki_info.uuid)
        .expect("create legacy uuid shadow");

    let main_info = main.get_repo_info().expect("main info").expect("present");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;
    let bad = deve_core::ledger::RepoInfo {
        uuid: main_info.uuid,
        name: "main".into(),
        url: Some(format!("urn:uuid:{}", main_info.uuid)),
    };
    let write = wiki_db.begin_write().expect("write txn");
    write
        .open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(
            &0,
            bincode::serialize(&bad)
                .expect("serialize repaired metadata")
                .as_slice(),
        )
        .expect("write bad metadata");
    write.commit().expect("commit metadata");

    main.repair_local_repo_catalog()
        .expect("repair local catalog");

    assert!(
        main.list_repos(Some(&peer_id))
            .expect_err("runtime listing must stay fail-closed after local-only repair")
            .to_string()
            .contains(
                format!("Broken shadow repo {} for peer {}", wiki_info.uuid, peer_id).as_str()
            )
    );
}

#[test]
fn remote_catalog_repair_does_not_borrow_local_metadata_for_shadow_naming() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 10, Some("main"), Some("urn:main")).expect("main");
    let wiki = RepoManager::init(&ledger_dir, 10, Some("wiki"), Some("urn:wiki")).expect("wiki");
    let peer_id = PeerId::new("peer-remote");
    let wiki_info = wiki.get_repo_info().expect("wiki info").expect("present");

    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;
    let poisoned = deve_core::ledger::RepoInfo {
        uuid: wiki_info.uuid,
        name: String::new(),
        url: wiki_info.url.clone(),
    };
    let write = wiki_db.begin_write().expect("write txn");
    write
        .open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(
            &0,
            bincode::serialize(&poisoned)
                .expect("serialize poisoned metadata")
                .as_slice(),
        )
        .expect("write poisoned metadata");
    write.commit().expect("commit metadata");

    main.ensure_shadow_db(&peer_id, &wiki_info.uuid)
        .expect("create legacy uuid shadow");
    main.repair_remote_repo_catalogs()
        .expect("repair remote catalogs");

    let selectors = main
        .list_repos(Some(&peer_id))
        .expect("list repaired shadows");
    assert_eq!(selectors, vec![wiki_info.uuid.to_string()]);
    let selector = selectors[0].clone();
    assert!(
        main.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", selector))
            .exists()
    );
}

#[test]
fn remote_catalog_repair_fails_closed_on_broken_peer() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let local = RepoManager::init(&ledger_dir, 10, Some("wiki"), Some("urn:test:wiki"))
        .expect("init local companion");
    let info = local
        .get_repo_info()
        .expect("local repo info")
        .expect("local repo exists");
    let good_peer = PeerId::new("peer-good");
    let bad_peer = PeerId::new("peer-bad");

    repo.ensure_shadow_db(&good_peer, &info.uuid)
        .expect("create healthy legacy uuid shadow");
    let bad_dir = repo.remotes_dir().join(bad_peer.to_filename());
    std::fs::create_dir_all(&bad_dir).expect("create bad peer dir");
    std::fs::write(bad_dir.join("broken.redb"), b"not-a-redb").expect("write broken shadow");

    let err = repo
        .repair_remote_repo_catalogs()
        .expect_err("broken peer must fail remote catalog repair");
    assert!(err.to_string().contains("Broken shadow peer peer-bad"));

    assert!(
        repo.list_repos(Some(&good_peer))
            .expect_err("healthy peer must stay unreadable until repair completes")
            .to_string()
            .contains(format!("Broken shadow repo {} for peer {}", info.uuid, good_peer).as_str())
    );
    let err = repo
        .list_shadows_on_disk()
        .expect_err("broken peer must fail shadow listing");
    assert!(err.to_string().contains("Broken shadow peer peer-bad"));
}
