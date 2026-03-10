use deve_core::ledger::RepoManager;
use deve_core::models::FileNodeId;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::scan;
use deve_core::utils::hash::StableHasher;
use deve_core::vfs::Vfs;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, repo)
}

fn workspace_path(dir: &TempDir, path: &str) -> std::path::PathBuf {
    dir.path().join("vault").join("default").join(path)
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
    let (dir, repo) = new_repo();
    let file = workspace_path(&dir, "notes/a.md");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, "hello").expect("write file");
    let repo = Arc::new(repo);
    let vfs = Vfs::new(dir.path().join("vault"));
    scan::scan_vault(&repo, &vfs, &dir.path().join("vault")).expect("scan initial");
    repo.stage_pending("notes/a.md").expect("stage file");
    repo.commit_staged("initial").expect("commit file");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");

    std::fs::remove_file(&file).expect("remove canonical");
    std::fs::write(workspace_path(&dir, "notes/b.md"), "hello").expect("write renamed file");
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

    let restored = workspace_path(&dir, "notes/a.md");
    let inode = inode_for(&restored);
    assert_eq!(
        repo.get_docid_by_inode_in_local_repo(repo.local_repo_name(), &inode)
            .expect("load inode binding"),
        Some(doc_id)
    );
}
