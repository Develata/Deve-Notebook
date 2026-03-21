use super::{RepoManager, cached_database};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
#[test]
fn cached_database_fails_closed_when_path_is_unstatable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let blocked = dir.path().join("blocked");
    std::fs::create_dir_all(&blocked).expect("blocked dir");
    let original = std::fs::metadata(&blocked).expect("metadata").permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(&blocked, perms).expect("chmod 000");
    let path = blocked.join("notes.redb");

    let err = cached_database(&path).expect_err("unstatable path must fail closed");

    std::fs::set_permissions(&blocked, original).expect("restore perms");
    assert!(
        err.to_string().contains("Failed to stat database path")
            || err.to_string().contains("Permission denied")
    );
}

#[cfg(unix)]
#[test]
fn open_local_database_fails_closed_when_path_is_unstatable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))
        .expect("repo");
    RepoManager::init(dir.path().join("ledger"), 8, Some("wiki"), Some("urn:wiki"))
        .expect("extra repo");
    let local_dir = dir.path().join("ledger/local");
    let original = std::fs::metadata(&local_dir)
        .expect("metadata")
        .permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(&local_dir, perms).expect("chmod 000");

    let err = match repo.open_database(None, "wiki") {
        Ok(_) => panic!("unstatable local db path must fail closed"),
        Err(err) => err,
    };

    std::fs::set_permissions(&local_dir, original).expect("restore perms");
    assert!(
        err.to_string()
            .contains("Failed to stat local repo directory")
            || err
                .to_string()
                .contains("Failed to stat local database path")
            || err.to_string().contains("Permission denied")
    );
}
