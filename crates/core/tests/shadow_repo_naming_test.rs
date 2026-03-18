use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use tempfile::TempDir;
use uuid::Uuid;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    (dir, repo)
}

#[test]
fn ensure_shadow_repo_info_renames_uuid_file() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let info = RepoInfo {
        uuid: repo_id,
        name: "notes".into(),
        url: Some("urn:test:notes".into()),
    };

    repo.ensure_shadow_db(&peer_id, &repo_id)
        .expect("create legacy shadow");
    repo.ensure_shadow_repo_info(&peer_id, &info)
        .expect("write shadow repo info");

    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    assert!(peer_dir.join("notes.redb").exists());
    assert!(!peer_dir.join(format!("{}.redb", repo_id)).exists());
}

#[test]
fn remote_repo_listing_prefers_metadata_name() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let info = RepoInfo {
        uuid: repo_id,
        name: "wiki".into(),
        url: Some("urn:test:wiki".into()),
    };

    repo.ensure_shadow_repo_info(&peer_id, &info)
        .expect("prepare named shadow");

    assert_eq!(
        repo.list_repos(Some(&peer_id)).expect("list remote repos"),
        vec!["wiki".to_string()]
    );
    let remote_info = repo
        .get_repo_info_for(Some(&peer_id), Some("wiki"))
        .expect("lookup remote repo info")
        .expect("remote repo info exists");
    assert_eq!(remote_info.uuid, repo_id);

    let handle = repo
        .open_database(Some(&peer_id), "wiki")
        .expect("open remote shadow by name");
    assert_eq!(handle.repo_name, "wiki");
}

#[test]
fn remote_repo_listing_reuses_open_remote_database() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let info = RepoInfo {
        uuid: repo_id,
        name: "wiki".into(),
        url: Some("urn:test:wiki".into()),
    };

    repo.ensure_shadow_repo_info(&peer_id, &info)
        .expect("prepare named shadow");
    let handle = repo
        .open_database(Some(&peer_id), "wiki")
        .expect("open remote shadow first");
    assert_eq!(handle.repo_name, "wiki");
    assert_eq!(
        repo.list_repos(Some(&peer_id)).expect("list remote repos"),
        vec!["wiki".to_string()]
    );
}

#[test]
fn remote_repo_lookup_keeps_uuid_selector_for_uuid_legacy_shadow() {
    let (dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let local = RepoManager::init(
        dir.path().join("ledger"),
        10,
        Some("wiki"),
        Some("urn:test:wiki"),
    )
    .expect("init local companion repo");
    let info = local
        .get_repo_info()
        .expect("local info")
        .expect("local repo exists");

    repo.ensure_shadow_db(&peer_id, &info.uuid)
        .expect("create legacy uuid shadow");

    assert_eq!(
        repo.list_repos(Some(&peer_id)).expect("list remote repos"),
        vec![info.uuid.to_string()]
    );
    let remote_info = repo
        .get_repo_info_for(Some(&peer_id), Some(&info.uuid.to_string()))
        .expect("lookup shadow by uuid")
        .expect("shadow info");
    assert_eq!(remote_info.name, info.uuid.to_string());
    let handle = repo
        .open_database(Some(&peer_id), &info.uuid.to_string())
        .expect("open remote shadow by uuid");
    assert_eq!(handle.repo_name, info.uuid.to_string());
}

#[test]
fn remote_repo_lookup_keeps_uuid_name_when_no_metadata_exists() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();

    repo.ensure_shadow_db(&peer_id, &repo_id)
        .expect("create legacy uuid shadow");

    assert_eq!(
        repo.list_repos(Some(&peer_id)).expect("list remote repos"),
        vec![repo_id.to_string()]
    );
    let remote_info = repo
        .get_repo_info_for(Some(&peer_id), Some(&repo_id.to_string()))
        .expect("lookup shadow by uuid")
        .expect("shadow info");
    assert_eq!(remote_info.uuid, repo_id);
    assert_eq!(remote_info.name, repo_id.to_string());
}

#[test]
fn ensure_shadow_repo_info_realigns_name_for_same_uuid() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();

    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "legacy".into(),
            url: Some("urn:test:wiki".into()),
        },
    )
    .expect("prepare legacy shadow");

    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "wiki".into(),
            url: Some("urn:test:wiki".into()),
        },
    )
    .expect("realign shadow name");

    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    assert!(peer_dir.join("wiki.redb").exists());
    assert!(!peer_dir.join("legacy.redb").exists());
    let info = repo
        .get_repo_info_for(Some(&peer_id), Some("wiki"))
        .expect("lookup remote repo info")
        .expect("remote repo info exists");
    assert_eq!(info.name, "wiki");
    assert_eq!(info.uuid, repo_id);
}
