use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use tempfile::TempDir;

#[test]
fn list_shadows_ignores_empty_peer_dirs() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-empty");
    std::fs::create_dir_all(repo.remotes_dir().join(peer_id.to_filename())).expect("peer dir");

    assert!(
        repo.list_shadows_on_disk()
            .expect("list shadows")
            .is_empty()
    );
}

#[test]
fn list_shadows_keeps_broken_peer_dirs_visible() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let good_peer = PeerId::new("peer-good");
    let bad_peer = PeerId::new("peer-bad");
    let info = deve_core::ledger::RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "notes".into(),
        url: Some("urn:test:notes".into()),
    };

    repo.ensure_shadow_repo_info(&good_peer, &info)
        .expect("seed good shadow");
    let bad_dir = repo.remotes_dir().join(bad_peer.to_filename());
    std::fs::create_dir_all(&bad_dir).expect("create bad peer dir");
    std::fs::write(bad_dir.join("broken.redb"), b"not-a-redb").expect("seed broken shadow");

    let shadows = repo.list_shadows_on_disk().expect("list shadows");
    assert_eq!(shadows, vec![bad_peer, good_peer]);
}

#[test]
fn broken_shadow_repos_stay_hidden_from_repo_listing_and_selector_recovery() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-mixed");
    let info = deve_core::ledger::RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "notes".into(),
        url: Some("urn:test:notes".into()),
    };

    repo.ensure_shadow_repo_info(&peer_id, &info)
        .expect("seed good shadow");
    std::fs::write(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join("broken.redb"),
        b"not-a-redb",
    )
    .expect("seed broken shadow");

    assert_eq!(
        repo.list_repos(Some(&peer_id)).expect("list repos"),
        vec!["notes"]
    );
    assert_eq!(
        repo.find_remote_repo_selector(&peer_id, "broken")
            .expect("resolve selector"),
        None
    );
}

#[test]
fn switchable_shadow_list_hides_peers_with_only_broken_repos() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let good_peer = PeerId::new("peer-good");
    let bad_peer = PeerId::new("peer-bad");
    let info = deve_core::ledger::RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "notes".into(),
        url: Some("urn:test:notes".into()),
    };

    repo.ensure_shadow_repo_info(&good_peer, &info)
        .expect("seed good shadow");
    let bad_dir = repo.remotes_dir().join(bad_peer.to_filename());
    std::fs::create_dir_all(&bad_dir).expect("create bad peer dir");
    std::fs::write(bad_dir.join("broken.redb"), b"not-a-redb").expect("seed broken shadow");

    assert_eq!(
        repo.list_switchable_shadows_on_disk()
            .expect("list switchable shadows"),
        vec![good_peer]
    );
}

#[test]
fn switchable_shadow_list_hides_peers_with_only_ambiguous_duplicate_uuid_shadows() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-dup");
    let info = deve_core::ledger::RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "notes".into(),
        url: Some("urn:test:notes".into()),
    };
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    for stem in ["notes", "notes-1"] {
        let db = redb::Database::create(peer_dir.join(format!("{stem}.redb"))).expect("shadow db");
        let write = db.begin_write().expect("write txn");
        write
            .open_table(deve_core::ledger::schema::REPO_METADATA)
            .expect("repo metadata")
            .insert(
                &0,
                bincode::serialize(&info)
                    .expect("serialize info")
                    .as_slice(),
            )
            .expect("write info");
        write.commit().expect("commit");
    }

    assert!(
        repo.list_repos(Some(&peer_id))
            .expect("list dup repos")
            .is_empty(),
        "duplicate uuid shadows must be hidden from repo listing"
    );
    assert!(
        repo.list_switchable_shadows_on_disk()
            .expect("list switchable shadows")
            .is_empty(),
        "duplicate uuid shadow peer must not be switchable"
    );
}

#[test]
fn pure_shadow_scan_does_not_resurrect_loaded_but_corrupted_shadow_files() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-corrupt");
    let info = deve_core::ledger::RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "notes".into(),
        url: Some("urn:test:notes".into()),
    };

    repo.ensure_shadow_repo_info(&peer_id, &info)
        .expect("seed shadow");
    std::fs::write(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join("notes.redb"),
        b"not-a-redb",
    )
    .expect("poison on-disk shadow");

    assert!(
        repo.list_repos(Some(&peer_id))
            .expect("list shadow repos after corruption")
            .is_empty(),
        "loaded in-memory metadata must not resurrect an unreadable shadow file",
    );
    assert!(
        repo.find_remote_repo_selector(&peer_id, "notes")
            .expect("resolve shadow selector after corruption")
            .is_none(),
        "corrupted shadow path must not stay selectable through loaded metadata",
    );
}
