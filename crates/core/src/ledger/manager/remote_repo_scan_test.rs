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
