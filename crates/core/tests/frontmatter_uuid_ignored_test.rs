use deve_core::models::DocId;
use deve_core::sync::SyncManager;
use tempfile::TempDir;

fn new_repo() -> (TempDir, std::sync::Arc<deve_core::ledger::RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = deve_core::ledger::RepoManager::init(dir.path().join("ledger"), 10, None, None)
        .expect("init repo");
    repo.set_projection_base_for_all_local_repos(dir.path().join("vault"));
    (dir, std::sync::Arc::new(repo))
}

#[test]
fn watcher_ignores_uuid_frontmatter_as_identity_source() {
    let (dir, repo) = new_repo();
    let doc_id = DocId::new();
    let file = dir
        .path()
        .join("vault")
        .join("default")
        .join("notes/new.md");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, format!("uuid: {doc_id}\nbody")).expect("write file");

    let sync = SyncManager::new(repo.clone());
    let repo_id = repo
        .get_repo_info_for(None, Some("default"))
        .expect("repo info lookup")
        .expect("repo info")
        .uuid;
    sync.handle_fs_event("default", repo_id, "notes/new.md")
        .expect("handle fs event");

    let pending = repo
        .list_pending_fs_in_local_repo(repo.local_repo_name())
        .expect("load pending");
    let new_file = pending
        .iter()
        .find(|entry| entry.path == "notes/new.md")
        .expect("new file pending");
    assert!(new_file.doc_id.is_none());
    assert!(pending.iter().all(|entry| entry.path != "notes/old.md"));
}
