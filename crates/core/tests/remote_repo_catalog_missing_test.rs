use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::shadow;
use deve_core::models::PeerId;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[test]
fn remote_repo_catalog_calls_fail_closed_when_remotes_dir_is_missing() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let peer_id = PeerId::new("peer-a");

    std::fs::remove_dir_all(ledger_dir.join("remotes")).expect("remove remotes catalog dir");

    let list_err = repo
        .list_shadows_on_disk()
        .expect_err("missing remotes dir must fail shadow listing");
    assert!(
        list_err
            .to_string()
            .contains("remote repo directory missing")
    );

    let switchable_err = repo
        .list_switchable_shadows_on_disk()
        .expect_err("missing remotes dir must fail switchable shadow listing");
    assert!(
        switchable_err
            .to_string()
            .contains("remote repo directory missing")
    );

    let repair_err = repo
        .repair_remote_repo_catalogs()
        .expect_err("missing remotes dir must fail remote repair");
    assert!(
        repair_err
            .to_string()
            .contains("remote repo directory missing")
    );

    let remote_list_err = repo
        .list_repos(Some(&peer_id))
        .expect_err("missing remotes dir must fail remote repo listing");
    assert!(
        remote_list_err
            .to_string()
            .contains("remote repo directory missing")
    );

    let selector_err = repo
        .find_remote_repo_selector(&peer_id, "notes")
        .expect_err("missing remotes dir must fail remote selector recovery");
    assert!(
        selector_err
            .to_string()
            .contains("remote repo directory missing")
    );

    let shadow_err = shadow::list_shadows_on_disk(&repo.remotes_dir())
        .expect_err("missing remotes dir must fail shadow management listing");
    assert!(
        shadow_err
            .to_string()
            .contains("remote repo directory missing")
    );
}

#[cfg(unix)]
#[test]
fn remote_repo_catalog_calls_fail_closed_when_peer_dir_is_unstatable() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let peer_id = PeerId::new("peer-a");
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let original = std::fs::metadata(&peer_dir)
        .expect("metadata")
        .permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&peer_dir, blocked).expect("chmod 000");

    let selector_err = repo
        .find_remote_repo_selector(&peer_id, "notes")
        .expect_err("unstatable peer dir must fail selector recovery");
    let list_err = repo
        .list_repos(Some(&peer_id))
        .expect_err("unstatable peer dir must fail remote repo listing");
    let repair_err = repo
        .repair_remote_repo_catalogs()
        .expect_err("unstatable peer dir must fail remote repair");

    std::fs::set_permissions(&peer_dir, original).expect("restore perms");
    assert!(
        selector_err
            .to_string()
            .contains("Failed to stat remote peer directory")
            || selector_err.to_string().contains("Permission denied")
    );
    assert!(
        list_err
            .to_string()
            .contains("Failed to stat remote peer directory")
            || list_err.to_string().contains("Permission denied")
    );
    assert!(
        repair_err
            .to_string()
            .contains("Broken shadow peer peer-a while repairing catalogs")
            || repair_err
                .to_string()
                .contains("Failed to stat remote peer directory")
            || repair_err.to_string().contains("Permission denied")
    );
}
