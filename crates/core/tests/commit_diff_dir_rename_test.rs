use deve_core::ledger::RepoManager;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, Arc<RepoManager>, std::path::PathBuf) {
    let dir = tempdir().expect("create tempdir");
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("projection base");
    let repo = Arc::new(repo);
    let workspace_root = repo
        .local_repo_workspace_root("default")
        .expect("workspace root");
    (dir, repo, workspace_root)
}

fn write_workspace_file(workspace_root: &std::path::Path, path: &str, content: &str) {
    let abs = workspace_root.join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn seed_pending_add(repo: &RepoManager, path: &str, content: &str) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending add");
}

#[test]
fn commit_diff_reports_child_rename_after_directory_move() {
    let (_dir, repo, workspace_root) = new_repo();
    write_workspace_file(&workspace_root, "notes/a.md", "hello");
    seed_pending_add(repo.as_ref(), "notes/a.md", "hello");
    repo.stage_pending("notes/a.md").expect("stage initial");
    let first = repo
        .commit_staged_with_git_bridge("initial", deve_core::config::GitBridgeMode::Mirror)
        .expect("commit initial");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup doc")
        .expect("doc id");

    std::fs::rename(workspace_root.join("notes"), workspace_root.join("docs")).expect("rename dir");
    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    let repo_id = repo
        .get_repo_info_for(None, Some("default"))
        .expect("repo info lookup")
        .expect("repo info")
        .uuid;
    sync.handle_dir_change("default", repo_id, "docs")
        .expect("handle dir change")
        .expect("repo-scoped result");
    repo.stage_pending("notes/a.md").expect("stage delete");
    repo.stage_pending("docs/a.md").expect("stage add");
    let second = repo
        .commit_staged_with_git_bridge("rename dir", deve_core::config::GitBridgeMode::Mirror)
        .expect("commit rename");

    let diffs = repo
        .diff_commits(Some(&first.id), &second.id)
        .expect("diff commits");
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].doc_id, Some(doc_id));
    assert_eq!(diffs[0].status, ChangeStatus::Renamed);
    assert_eq!(diffs[0].previous_path.as_deref(), Some("notes/a.md"));
    assert_eq!(diffs[0].path, "docs/a.md");
    assert_eq!(diffs[0].old_content, "hello");
    assert_eq!(diffs[0].new_content, "hello");
}
