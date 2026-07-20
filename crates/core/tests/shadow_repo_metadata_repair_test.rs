use deve_core::codec;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::schema::{REPO_INFO_METADATA_KEY, REPO_METADATA};
use deve_core::models::PeerId;
use tempfile::TempDir;

mod common;

fn read_repo_info(db: &redb::Database) -> Option<deve_core::ledger::RepoInfo> {
    let read = db.begin_read().expect("read txn");
    let table = read.open_table(REPO_METADATA).expect("repo metadata");
    let raw = table.get(&REPO_INFO_METADATA_KEY).expect("read metadata")?;
    Some(codec::decode(raw.value()).expect("decode repo info"))
}

#[test]
fn remote_catalog_keeps_legacy_uuid_shadow_non_switchable_without_remote_metadata() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let info = common::create_initialized_local_repo_with_depth(
        &dir.path().join("ledger"),
        10,
        "wiki",
        "urn:test:wiki",
    );
    let peer_id = PeerId::new("peer-remote");

    common::seed_shadow_without_metadata_row(&repo, &peer_id, info.uuid);
    repo.repair_remote_repo_catalogs()
        .expect("repair remote catalogs explicitly");
    assert!(
        repo.list_repos(Some(&peer_id))
            .expect("list repaired shadows")
            .is_empty()
    );
    assert_eq!(
        repo.find_remote_repo_selector_by_id(&peer_id, info.uuid)
            .expect("resolve repaired shadow selector"),
        None
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
fn init_keeps_uuid_shadow_non_switchable_without_remote_metadata() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let info =
        common::create_initialized_local_repo_with_depth(&ledger_dir, 10, "wiki", "urn:test:wiki");
    let peer_id = PeerId::new("peer-remote");

    common::seed_shadow_without_metadata_row(&repo, &peer_id, info.uuid);
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );

    let repaired = RepoManager::init(&ledger_dir, 10, None, None).expect("re-init repo");
    assert!(
        repaired
            .list_repos(Some(&peer_id))
            .expect("list repaired shadows")
            .is_empty()
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
    let _repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-bad");
    common::seed_broken_remote_shadow_repo(&ledger_dir, &peer_id, "broken");

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
    let wiki_info =
        common::create_initialized_local_repo_with_depth(&ledger_dir, 10, "wiki", "urn:wiki");
    let peer_id = PeerId::new("peer-remote");

    common::seed_shadow_without_metadata_row(&main, &peer_id, wiki_info.uuid);

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
    let (main, _main_id) =
        common::init_cataloged_repo(&ledger_dir, &dir.path().join("main-notes")).expect("main");
    let (_wiki, wiki_id) = common::init_cataloged_repo_with_url(
        &ledger_dir,
        &dir.path().join("wiki-notes"),
        "urn:wiki",
    )
    .expect("wiki");
    let peer_id = PeerId::new("peer-remote");

    let wiki_db = main
        .open_database(None, &wiki_id.to_string())
        .expect("wiki db")
        .db;
    let poisoned = deve_core::ledger::RepoInfo {
        uuid: wiki_id,
        name: String::new(),
        url: Some("urn:wiki".to_string()),
    };
    common::write_repo_metadata(wiki_db.as_ref(), &poisoned);

    common::seed_shadow_without_metadata_row(&main, &peer_id, wiki_id);
    main.repair_remote_repo_catalogs()
        .expect("repair remote catalogs");

    assert!(
        main.list_repos(Some(&peer_id))
            .expect("list repaired shadows")
            .is_empty()
    );
    assert!(
        main.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", wiki_id))
            .exists()
    );
}

#[test]
fn remote_catalog_repair_fails_closed_on_broken_peer() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let info =
        common::create_initialized_local_repo_with_depth(&ledger_dir, 10, "wiki", "urn:test:wiki");
    let good_peer = PeerId::new("peer-good");
    let bad_peer = PeerId::new("peer-bad");

    common::seed_shadow_without_metadata_row(&repo, &good_peer, info.uuid);
    common::seed_broken_remote_shadow_repo(&ledger_dir, &bad_peer, "broken");

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
