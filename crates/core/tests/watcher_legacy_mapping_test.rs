use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::DocId;
use deve_core::sync::SyncManager;
use deve_core::sync::scan::scan_projection_workspaces;
use deve_core::vfs::Vfs;
use std::sync::Arc;
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, Arc<RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) =
        common::init_cataloged_repo(&dir.path().join("ledger"), &dir.path().join("notes"))
            .expect("init cataloged repo");
    (dir, Arc::new(repo))
}

#[test]
fn watcher_treats_legacy_only_path_as_new_file() {
    let (_dir, repo) = new_repo();
    let path = repo
        .local_repo_workspace_path(repo.local_repo_name(), "notes/legacy.md")
        .expect("workspace path");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, "legacy content").expect("write file");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let doc_id = DocId::new();
        let write = db.begin_write()?;
        {
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/legacy.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/legacy.md")?;
        }
        write.commit()?;
        Ok::<_, anyhow::Error>(())
    })
    .expect("seed legacy mapping");

    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    let repo_id = repo
        .get_repo_info_for(None, Some(repo.local_repo_name()))
        .expect("repo info lookup")
        .expect("repo info")
        .uuid;
    let messages = sync
        .handle_fs_event(repo.local_repo_name(), repo_id, "notes/legacy.md")
        .expect("legacy-only path treated as new file");

    assert!(
        !messages.is_empty(),
        "watcher must produce pending added event for legacy-only path"
    );
}

#[test]
fn full_scan_treats_legacy_only_path_as_new_file() {
    let (_dir, repo) = new_repo();
    let path = repo
        .local_repo_workspace_path(repo.local_repo_name(), "notes/legacy.md")
        .expect("workspace path");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, "legacy content").expect("write file");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let doc_id = DocId::new();
        let write = db.begin_write()?;
        {
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/legacy.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/legacy.md")?;
        }
        write.commit()?;
        Ok::<_, anyhow::Error>(())
    })
    .expect("seed legacy mapping");

    let vfs = Vfs::new(
        repo.local_repo_workspace_root(repo.local_repo_name())
            .expect("workspace root"),
    );
    scan_projection_workspaces(&repo, &vfs)
        .expect("full scan succeeds; legacy-only path treated as new");

    let pending = repo
        .list_pending_fs_in_local_repo(repo.local_repo_name())
        .expect("pending list");
    assert!(
        !pending.is_empty(),
        "full scan must create pending entry for legacy-only path"
    );
}
