use deve_core::ledger::RepoManager;
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn scan_fails_closed_on_unreadable_repo_dir() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let vault_dir = dir.path().join("vault");
    let mut repo =
        RepoManager::init(&ledger_dir, 10, Some("default"), Some("urn:default")).expect("init");
    repo.set_projection_base_for_all_local_repos(&vault_dir);
    let repo = Arc::new(repo);
    let sync = SyncManager::new(repo);

    let unreadable = vault_dir.join("default").join("private");
    std::fs::create_dir_all(&unreadable).expect("create unreadable dir");
    std::fs::write(unreadable.join("hidden.md"), "# hidden").expect("write hidden doc");
    let mut perms = std::fs::metadata(&unreadable)
        .expect("metadata")
        .permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&unreadable, perms).expect("chmod 000");

    let err = sync.scan().expect_err("scan must fail closed");
    assert!(
        err.to_string()
            .contains("Failed to walk local repo default")
    );

    let mut restore = std::fs::metadata(&unreadable)
        .expect("metadata")
        .permissions();
    restore.set_mode(0o755);
    std::fs::set_permissions(&unreadable, restore).expect("restore perms");
}

#[test]
fn scan_fails_closed_on_markdown_path_that_is_not_a_file() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let vault_dir = dir.path().join("vault");
    let mut repo =
        RepoManager::init(&ledger_dir, 10, Some("default"), Some("urn:default")).expect("init");
    repo.set_projection_base_for_all_local_repos(&vault_dir);
    let repo = Arc::new(repo);
    let sync = SyncManager::new(repo);

    std::fs::create_dir_all(vault_dir.join("default").join("broken.md"))
        .expect("create invalid markdown directory");

    let err = sync
        .scan()
        .expect_err("non-file markdown path must fail closed");
    assert!(err.to_string().contains("markdown path is not a file"));
}

#[test]
fn scan_ignores_git_mirror_markdown_paths() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let vault_dir = dir.path().join("vault");
    let mut repo =
        RepoManager::init(&ledger_dir, 10, Some("default"), Some("urn:default")).expect("init");
    repo.set_projection_base_for_all_local_repos(&vault_dir);
    let repo = Arc::new(repo);
    let sync = SyncManager::new(repo.clone());

    let internal = vault_dir.join("default/.git/objects/x.md");
    std::fs::create_dir_all(internal.parent().expect("parent")).expect("mkdir");
    std::fs::write(&internal, "git mirror state").expect("write");

    sync.scan().expect("scan");

    assert!(
        repo.list_pending_fs_in_local_repo("default")
            .unwrap()
            .is_empty()
    );
}
