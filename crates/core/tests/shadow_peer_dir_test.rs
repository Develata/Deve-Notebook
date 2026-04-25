use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use tempfile::TempDir;

mod common;

#[cfg(unix)]
#[test]
fn invalid_shadow_peer_dir_name_fails_closed_for_listing_and_repair() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let invalid_peer = repo
        .remotes_dir()
        .join(OsString::from_vec(vec![0xff, b'p', b'e', b'e', b'r']));
    std::fs::create_dir_all(&invalid_peer).expect("create invalid peer dir");

    let list_err = repo
        .list_shadows_on_disk()
        .expect_err("invalid peer dir must fail closed");
    assert!(list_err.to_string().contains("invalid directory name"));

    let repair_err = repo
        .repair_remote_repo_catalogs()
        .expect_err("invalid peer dir must fail repair");
    assert!(repair_err.to_string().contains("invalid directory name"));
}

#[test]
fn non_directory_shadow_peer_entry_fails_closed_for_listing_and_repair() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let invalid_peer = repo.remotes_dir().join("peer-file");
    std::fs::write(&invalid_peer, b"not-a-directory").expect("create invalid peer file");

    let list_err = repo
        .list_shadows_on_disk()
        .expect_err("non-directory peer entry must fail closed");
    assert!(list_err.to_string().contains("expected directory"));

    let repair_err = repo
        .repair_remote_repo_catalogs()
        .expect_err("non-directory peer entry must fail repair");
    assert!(repair_err.to_string().contains("expected directory"));
}

#[test]
fn hidden_shadow_peer_dir_fails_closed_for_listing_and_repair() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    std::fs::create_dir_all(repo.remotes_dir().join(".peer-hidden")).expect("create hidden peer");

    let list_err = repo
        .list_shadows_on_disk()
        .expect_err("hidden peer dir must fail closed");
    assert!(list_err.to_string().contains("unexpected hidden directory"));

    let repair_err = repo
        .repair_remote_repo_catalogs()
        .expect_err("hidden peer dir must fail repair");
    assert!(
        repair_err
            .to_string()
            .contains("unexpected hidden directory")
    );
}

#[test]
fn non_file_shadow_repo_entry_fails_closed_for_listing_and_repair() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer = deve_core::models::PeerId::new("peer-dir");
    common::seed_non_file_shadow_repo_entry(&repo, &peer, "notes");

    let list_err = repo
        .list_repos(Some(&peer))
        .expect_err("non-file shadow entry must fail listing");
    assert!(list_err.to_string().contains("expected file"));

    let repair_err = repo
        .repair_remote_repo_catalogs()
        .expect_err("non-file shadow entry must fail repair");
    assert!(
        repair_err
            .to_string()
            .contains("Broken shadow peer peer-dir while repairing catalogs")
    );
}

#[test]
fn hidden_non_redb_shadow_repo_entry_fails_closed_for_listing_and_repair() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer = deve_core::models::PeerId::new("peer-hidden-entry");
    let peer_dir = repo.remotes_dir().join(peer.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("create peer dir");
    std::fs::write(peer_dir.join(".stale"), b"shadow-junk").expect("create hidden junk");

    let list_err = repo
        .list_repos(Some(&peer))
        .expect_err("hidden non-redb shadow entry must fail listing");
    assert!(list_err.to_string().contains("unexpected non-redb entry"));

    let repair_err = repo
        .repair_remote_repo_catalogs()
        .expect_err("hidden non-redb shadow entry must fail repair");
    let detail = repair_err.to_string();
    assert!(
        detail.contains("Broken shadow peer peer-hidden-entry while repairing catalogs")
            || detail.contains("unexpected non-redb entry"),
        "unexpected repair error: {detail}"
    );
}
