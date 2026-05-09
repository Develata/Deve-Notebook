use crate::ledger::RepoManager;
use crate::models::PeerId;

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
    crate::test_support::seed_shadow_repo_missing_metadata(&repo, "peer-a", "broken");

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
    crate::test_support::seed_shadow_repo_missing_metadata(&repo, "peer-a", "broken");

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
    crate::test_support::seed_shadow_repo_missing_metadata(&repo, "peer-a", "broken");

    let err = repo
        .has_remote_display_name(&peer, "broken")
        .expect_err("unreadable shadow metadata must fail closed");

    assert!(
        err.to_string().contains("Broken shadow repo"),
        "unexpected error: {err}"
    );
}
