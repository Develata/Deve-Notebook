use deve_core::ledger::RepoManager;
use deve_core::sync::SyncManager;
use tempfile::TempDir;

fn new_repo() -> (TempDir, std::sync::Arc<RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, std::sync::Arc::new(repo))
}

#[test]
fn materialize_projection_creates_empty_directories_from_structure_facts() {
    let (dir, repo) = new_repo();
    repo.apply_dir_create_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/archive/2026",
        "test",
    )
    .expect("create dir structure");

    let sync = SyncManager::new(repo, dir.path().join("vault"));
    sync.materialize_local_repo("default").expect("materialize");

    let root = dir.path().join("vault/default");
    assert!(root.join("notes").is_dir());
    assert!(root.join("notes/archive").is_dir());
    assert!(root.join("notes/archive/2026").is_dir());
}
