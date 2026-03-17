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
fn remote_repo_listing_uses_collision_safe_labels_for_duplicate_names() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:test:wiki-a".into()),
        },
    )
    .expect("prepare first wiki shadow");
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:test:wiki-b".into()),
        },
    )
    .expect("prepare second wiki shadow");

    let repos = repo.list_repos(Some(&peer_id)).expect("list remote repos");
    assert_eq!(repos.len(), 2);
    assert!(repos.iter().any(|name| name == "wiki"));
    assert!(
        repos
            .iter()
            .any(|name| name != "wiki" && name.starts_with("wiki-"))
    );
}

#[test]
fn remote_repo_selector_stays_stable_for_same_uuid_under_name_collision() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let first = RepoInfo {
        uuid: Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-a".into()),
    };
    let second = RepoInfo {
        uuid: Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-b".into()),
    };
    repo.ensure_shadow_repo_info(&peer_id, &first)
        .expect("prepare first wiki shadow");
    repo.ensure_shadow_repo_info(&peer_id, &second)
        .expect("prepare second wiki shadow");

    let before = repo
        .find_remote_repo_selector_by_id(&peer_id, second.uuid)
        .expect("lookup selector")
        .expect("selector must exist");
    repo.ensure_shadow_repo_info(&peer_id, &second)
        .expect("rewrite same shadow info");
    let after = repo
        .find_remote_repo_selector_by_id(&peer_id, second.uuid)
        .expect("lookup selector after rewrite")
        .expect("selector must still exist");

    assert_eq!(before, after);
}

#[test]
fn open_remote_database_preserves_collision_safe_selector() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let first = RepoInfo {
        uuid: Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-a".into()),
    };
    let second = RepoInfo {
        uuid: Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-b".into()),
    };
    repo.ensure_shadow_repo_info(&peer_id, &first)
        .expect("prepare first wiki shadow");
    repo.ensure_shadow_repo_info(&peer_id, &second)
        .expect("prepare second wiki shadow");

    let selector = repo
        .find_remote_repo_selector_by_id(&peer_id, second.uuid)
        .expect("lookup collision-safe selector")
        .expect("selector exists");
    let handle = repo
        .open_database(Some(&peer_id), &selector)
        .expect("open remote shadow by exact selector");

    assert_eq!(handle.repo_name, selector);
    assert_eq!(handle.repo_id, Some(second.uuid));
}

#[test]
fn remote_repo_selector_can_recover_from_uuid_string() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let info = RepoInfo {
        uuid: Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-a".into()),
    };
    repo.ensure_shadow_repo_info(&peer_id, &info)
        .expect("prepare shadow repo");

    let selector = repo
        .find_remote_repo_selector(&peer_id, &info.uuid.to_string())
        .expect("resolve selector")
        .expect("selector exists");

    assert_eq!(
        selector,
        repo.find_remote_repo_selector_by_id(&peer_id, info.uuid)
            .expect("selector by id")
            .expect("selector exists")
    );
}

#[test]
fn remote_repo_selector_prefers_exact_uuid_shaped_stem_over_foreign_uuid_match() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let exact_selector = Uuid::new_v4();
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: Uuid::new_v4(),
            name: exact_selector.to_string(),
            url: Some("urn:test:uuid-shaped-selector".into()),
        },
    )
    .expect("prepare exact uuid-shaped selector");
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: exact_selector,
            name: "notes-b".into(),
            url: Some("urn:test:notes-b".into()),
        },
    )
    .expect("prepare foreign uuid match");

    assert_eq!(
        repo.find_remote_repo_selector(&peer_id, &exact_selector.to_string())
            .expect("resolve selector"),
        Some(exact_selector.to_string())
    );
}

#[test]
fn remote_repo_selector_by_id_does_not_match_uuid_shaped_display_name() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let target_uuid = Uuid::new_v4();
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: Uuid::new_v4(),
            name: target_uuid.to_string(),
            url: Some("urn:test:uuid-shaped-name".into()),
        },
    )
    .expect("prepare uuid-shaped display-name shadow");

    assert_eq!(
        repo.find_remote_repo_selector_by_id(&peer_id, target_uuid)
            .expect("lookup by id must not guess from display name"),
        None,
    );
}
