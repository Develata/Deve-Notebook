use deve_core::ledger::RepoManager;
use deve_core::models::FileNodeId;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::scan;
use deve_core::utils::hash::StableHasher;
use deve_core::vfs::Vfs;
use std::hash::{Hash, Hasher};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, repo)
}

fn workspace_path(repo: &RepoManager, path: &str) -> std::path::PathBuf {
    repo.local_repo_workspace_path("default", path)
        .expect("workspace path")
}

fn inode_for(path: &std::path::Path) -> FileNodeId {
    let file_id = file_id::get_file_id(path).expect("file id");
    let mut hasher = StableHasher::new();
    file_id.hash(&mut hasher);
    FileNodeId {
        id: hasher.finish() as u128,
    }
}

#[test]
fn discard_tracked_add_rebinds_workspace_inode() {
    let (_dir, repo) = new_repo();
    let file = workspace_path(&repo, "notes/a.md");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, "hello").expect("write file");
    let repo_root = repo
        .local_repo_workspace_root("default")
        .expect("workspace root");
    let repo = Arc::new(repo);
    let vfs = Vfs::new(repo_root);
    scan::scan_projection_workspaces(&repo, &vfs).expect("scan initial");
    repo.stage_pending("notes/a.md").expect("stage file");
    repo.commit_staged("initial").expect("commit file");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");

    std::fs::remove_file(&file).expect("remove canonical");
    std::fs::write(workspace_path(&repo, "notes/b.md"), "hello").expect("write renamed file");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed tracked add");

    repo.discard_pending("notes/b.md")
        .expect("discard tracked add");

    let restored = workspace_path(&repo, "notes/a.md");
    let inode = inode_for(&restored);
    assert_eq!(
        repo.get_docid_by_inode_in_local_repo(repo.local_repo_name(), &inode)
            .expect("load inode binding"),
        Some(doc_id)
    );
}

#[cfg(unix)]
#[test]
fn discard_tracked_add_fails_closed_on_unstatable_workspace_path() {
    let (_dir, repo) = new_repo();
    let file = workspace_path(&repo, "notes/a.md");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, "hello").expect("write file");
    let repo_root = repo
        .local_repo_workspace_root("default")
        .expect("workspace root");
    let repo = Arc::new(repo);
    let vfs = Vfs::new(repo_root);
    scan::scan_projection_workspaces(&repo, &vfs).expect("scan initial");
    repo.stage_pending("notes/a.md").expect("stage file");
    repo.commit_staged("initial").expect("commit file");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");

    std::fs::remove_file(&file).expect("remove canonical");
    std::fs::write(workspace_path(&repo, "notes/b.md"), "hello").expect("write renamed file");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed tracked add");

    let notes_dir = workspace_path(&repo, "notes");
    let original = std::fs::metadata(&notes_dir)
        .expect("metadata")
        .permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&notes_dir, blocked).expect("chmod 000");

    let err = repo
        .discard_pending("notes/b.md")
        .expect_err("unstatable tracked add must fail closed");

    std::fs::set_permissions(&notes_dir, original).expect("restore perms");
    assert!(
        err.to_string().contains("Failed to stat workspace path")
            || err.to_string().contains("Permission denied")
    );
    let pending = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            pending_fs::get(db, "notes/b.md")
        })
        .expect("load pending");
    assert!(pending.is_some());
}
