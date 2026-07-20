use deve_core::models::DocId;
use deve_core::sync::SyncManager;
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, std::sync::Arc<deve_core::ledger::RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init cataloged repo");
    (dir, std::sync::Arc::new(repo))
}

#[test]
fn watcher_ignores_uuid_frontmatter_as_identity_source() {
    let (_dir, repo) = new_repo();
    let doc_id = DocId::new();
    let file = repo
        .local_repo_workspace_path(repo.local_repo_name(), "notes/new.md")
        .expect("workspace path");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, format!("uuid: {doc_id}\nbody")).expect("write file");

    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    let repo_id = repo
        .get_repo_info_for(None, Some(repo.local_repo_name()))
        .expect("repo info lookup")
        .expect("repo info")
        .uuid;
    sync.handle_fs_event(repo.local_repo_name(), repo_id, "notes/new.md")
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
