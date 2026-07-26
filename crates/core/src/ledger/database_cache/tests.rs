use super::{OPENED_DBS, evict_database_paths_under, register_database, reusable_cached_database};
#[cfg(unix)]
use crate::ledger::RepoManager;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

fn clear_temp_entries(root: &std::path::Path) {
    evict_database_paths_under(root).expect("evict temp cache entries");
}

#[cfg(unix)]
#[test]
fn reusable_cached_database_fails_closed_when_path_is_unstatable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let blocked = dir.path().join("blocked");
    std::fs::create_dir_all(&blocked).expect("mkdir");
    let original = std::fs::metadata(&blocked).expect("metadata").permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(&blocked, perms).expect("chmod 000");

    let err = reusable_cached_database(&blocked.join("notes.redb"))
        .expect_err("unstatable path must fail closed");

    std::fs::set_permissions(&blocked, original).expect("restore perms");
    assert!(
        err.to_string()
            .contains("Failed to read database cache metadata")
            || err.to_string().contains("Permission denied")
    );
}

#[cfg(unix)]
#[test]
fn register_database_fails_closed_when_path_is_unstatable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = Arc::new(redb::Database::create(dir.path().join("main.redb")).expect("create db"));
    let blocked = dir.path().join("blocked");
    std::fs::create_dir_all(&blocked).expect("mkdir");
    let original = std::fs::metadata(&blocked).expect("metadata").permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(&blocked, perms).expect("chmod 000");

    let err = register_database(&blocked.join("notes.redb"), db).expect_err("must fail closed");

    std::fs::set_permissions(&blocked, original).expect("restore perms");
    clear_temp_entries(dir.path());
    assert!(
        err.to_string()
            .contains("Failed to read database cache metadata")
            || err.to_string().contains("Permission denied")
    );
}

#[cfg(unix)]
#[test]
fn reusable_cached_database_fails_closed_when_header_cannot_be_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("main.redb");
    let db = Arc::new(redb::Database::create(&path).expect("create db"));
    register_database(&path, db).expect("register");

    let stamp = {
        let cache = OPENED_DBS.read().expect("cache");
        cache
            .get(&path)
            .and_then(|entry| entry.stamp)
            .expect("registered stamp")
    };
    {
        let mut cache = OPENED_DBS.write().expect("cache");
        let entry = cache.get_mut(&path).expect("cached entry");
        let mut stale = stamp;
        stale.modified = None;
        entry.stamp = Some(stale);
    }

    let original = std::fs::metadata(&path).expect("metadata").permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&path, blocked).expect("chmod 000");

    let err = reusable_cached_database(&path).expect_err("unreadable redb header must fail closed");

    std::fs::set_permissions(&path, original).expect("restore perms");
    clear_temp_entries(dir.path());
    assert!(
        err.to_string().contains("Failed to open database header")
            || err.to_string().contains("Failed to read database header")
            || err.to_string().contains("Permission denied")
    );
}

#[cfg(unix)]
#[test]
fn local_catalog_validation_uses_authority_handle_without_global_cache() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("repo");
    let path = ledger_dir
        .join("local")
        .join(format!("{}.redb", repo.local_repo_name()));

    {
        let cache = OPENED_DBS.read().expect("cache");
        assert!(
            !cache.contains_key(&path),
            "local authority databases must not enter the shadow database cache"
        );
    }

    let original = std::fs::metadata(&path).expect("metadata").permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&path, blocked).expect("chmod 000");

    repo.refresh_local_repo_catalog()
        .expect("main repo validation must use the open database handle");
    {
        let cache = OPENED_DBS.read().expect("cache");
        assert!(
            !cache.contains_key(&path),
            "catalog refresh must not register local authority in the shadow database cache"
        );
    }

    std::fs::set_permissions(&path, original).expect("restore perms");
    clear_temp_entries(dir.path());
}

#[cfg(windows)]
#[test]
fn reusable_cached_database_reuses_locked_same_file_on_windows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("main.redb");
    let db = Arc::new(redb::Database::create(&path).expect("create db"));
    register_database(&path, db.clone()).expect("register");

    let stamp = {
        let cache = OPENED_DBS.read().expect("cache");
        cache
            .get(&path)
            .and_then(|entry| entry.stamp)
            .expect("registered stamp")
    };
    {
        let mut cache = OPENED_DBS.write().expect("cache");
        let entry = cache.get_mut(&path).expect("cached entry");
        let mut stale = stamp;
        stale.len = stale.len.saturating_add(1);
        entry.stamp = Some(stale);
    }

    let reused = reusable_cached_database(&path)
        .expect("stale same-file cache entry should reuse the open redb handle")
        .expect("cached database");

    assert!(Arc::ptr_eq(&reused, &db));
    clear_temp_entries(dir.path());
}
