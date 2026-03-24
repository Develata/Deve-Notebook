use crate::ledger::RepoManager;
use crate::models::PeerId;

/// 在 peer 目录下创建一个有效 .redb 文件但无 REPO_METADATA 表，
/// 使 `read_repo_info_from_path` 返回 `Ok(None)` → `!is_readable()`。
fn seed_unreadable_shadow(repo: &RepoManager, peer_name: &str, stem: &str) {
    let peer_dir = repo.remotes_dir().join(peer_name);
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let path = peer_dir.join(format!("{stem}.redb"));
    drop(redb::Database::create(&path).expect("empty db"));
}

#[test]
fn repair_remote_repo_catalog_fails_closed_on_missing_peer_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))
        .expect("repo");
    let peer = PeerId::new("peer-missing");

    let err = repo
        .repair_remote_repo_catalog(&peer)
        .expect_err("missing peer directory must fail closed");

    assert!(
        err.to_string()
            .contains("Broken shadow peer peer-missing while repairing catalog: directory missing")
    );
}

#[test]
fn resolve_remote_repo_entry_fails_closed_on_unreadable_shadow_metadata() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))
        .expect("repo");
    let peer = PeerId::new("peer-a");
    seed_unreadable_shadow(&repo, "peer-a", "broken");

    let err = repo
        .resolve_remote_repo_entry(&peer, "broken")
        .expect_err("unreadable shadow metadata must fail closed");

    assert!(
        err.to_string().contains("Broken shadow repo"),
        "unexpected error: {err}"
    );
}

#[test]
fn list_remote_repo_names_fails_closed_on_unreadable_shadow_metadata() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))
        .expect("repo");
    let peer = PeerId::new("peer-a");
    seed_unreadable_shadow(&repo, "peer-a", "broken");

    let err = repo
        .list_remote_repo_names(&peer)
        .expect_err("unreadable shadow metadata must fail closed");

    assert!(
        err.to_string().contains("Broken shadow repo"),
        "unexpected error: {err}"
    );
}

#[test]
fn has_remote_display_name_fails_closed_on_unreadable_shadow_metadata() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))
        .expect("repo");
    let peer = PeerId::new("peer-a");
    seed_unreadable_shadow(&repo, "peer-a", "broken");

    let err = repo
        .has_remote_display_name(&peer, "broken")
        .expect_err("unreadable shadow metadata must fail closed");

    assert!(
        err.to_string().contains("Broken shadow repo"),
        "unexpected error: {err}"
    );
}
