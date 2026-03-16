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
fn list_shadows_fail_closed_on_broken_peer_dirs() {
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

    let err = repo
        .list_shadows_on_disk()
        .expect_err("broken shadow peer must fail closed");
    assert!(err.to_string().contains("Broken shadow peer peer-bad"));
}

#[test]
fn broken_shadow_repos_fail_closed_in_repo_listing_and_stay_hidden_from_selector_recovery() {
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

    let err = repo
        .list_repos(Some(&peer_id))
        .expect_err("broken shadow repo listing must fail closed");
    assert!(
        err.to_string()
            .contains("Broken shadow repo broken for peer peer-mixed")
    );
    let selector_err = repo
        .find_remote_repo_selector(&peer_id, "broken")
        .expect_err("broken selector resolution must fail closed");
    assert!(
        selector_err
            .to_string()
            .contains("Broken shadow repo broken for peer peer-mixed")
    );
}

#[test]
fn switchable_shadow_list_fails_closed_on_broken_repos() {
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

    let err = repo
        .list_switchable_shadows_on_disk()
        .expect_err("broken shadow peer must fail switchable listing");
    assert!(
        err.to_string()
            .contains("Broken shadow repo broken for peer peer-bad")
    );
}

#[test]
fn switchable_shadow_list_is_sorted_stably() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let later_peer = PeerId::new("peer-z");
    let earlier_peer = PeerId::new("peer-a");
    let info = deve_core::ledger::RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "notes".into(),
        url: Some("urn:test:notes".into()),
    };

    repo.ensure_shadow_repo_info(&later_peer, &info)
        .expect("seed later peer");
    repo.ensure_shadow_repo_info(
        &earlier_peer,
        &deve_core::ledger::RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "notes".into(),
            url: Some("urn:test:notes-2".into()),
        },
    )
    .expect("seed earlier peer");

    assert_eq!(
        repo.list_switchable_shadows_on_disk()
            .expect("list switchable shadows"),
        vec![earlier_peer, later_peer]
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
fn pure_shadow_scan_fails_closed_and_does_not_resurrect_loaded_corrupted_shadow_files() {
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

    let err = repo
        .list_repos(Some(&peer_id))
        .expect_err("corrupted shadow list must fail closed");
    assert!(
        err.to_string()
            .contains("Broken shadow repo notes for peer peer-corrupt")
    );
    let selector_err = repo
        .find_remote_repo_selector(&peer_id, "notes")
        .expect_err("corrupted shadow selector must fail closed");
    assert!(
        selector_err
            .to_string()
            .contains("Broken shadow repo notes for peer peer-corrupt")
    );
}

#[cfg(unix)]
#[test]
fn shadow_listing_fails_closed_on_invalid_repo_stem() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-invalid");
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let invalid_path = peer_dir.join(OsString::from_vec(vec![0xff, b'.', b'r', b'e', b'd', b'b']));
    let invalid = redb::Database::create(&invalid_path).expect("invalid shadow db");
    invalid.begin_write().expect("write txn").commit().expect("commit");
    drop(invalid);

    let err = repo
        .list_repos(Some(&peer_id))
        .expect_err("invalid shadow stem must fail closed");
    assert!(err.to_string().contains("invalid file stem"));
}
