use deve_core::ledger::RepoManager;
use deve_core::models::FileNodeId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::{SyncManager, scan};
use deve_core::utils::hash::StableHasher;
use deve_core::vfs::Vfs;
use std::hash::{Hash, Hasher};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};

mod common;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init cataloged repo");
    (dir, repo)
}

fn workspace_path(repo: &RepoManager, path: &str) -> std::path::PathBuf {
    repo.local_repo_workspace_path(repo.local_repo_name(), path)
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

fn commit_initial_file(repo: RepoManager, path: &str, content: &str) -> Arc<RepoManager> {
    let file = workspace_path(&repo, path);
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, content).expect("write file");
    let repo_root = repo
        .local_repo_workspace_root(repo.local_repo_name())
        .expect("workspace root");
    let repo = Arc::new(repo);
    let vfs = Vfs::new(repo_root);
    scan::scan_projection_workspaces(&repo, &vfs).expect("scan initial");
    repo.stage_pending(path).expect("stage file");
    repo.apply_external_changes().expect("apply external file");
    repo.commit_source_control_changes("initial")
        .expect("commit file");
    repo
}

fn seed_docless_added_pending(repo: &RepoManager, path: &str, content: &str) {
    let file = workspace_path(repo, path);
    std::fs::write(&file, content).expect("write replacement");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash(content),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed docless added pending entry");
}

fn assert_docless_added_guard(err: &anyhow::Error) {
    assert!(
        err.to_string()
            .contains("Docless added pending entry points at tracked path"),
        "{err}"
    );
}

fn assert_pending_and_file_preserved(repo: &RepoManager, path: &str, content: &str) {
    assert_eq!(
        std::fs::read_to_string(workspace_path(repo, path)).expect("read workspace file"),
        content
    );
    let pending = repo
        .run_on_local_repo(repo.local_repo_name(), |db| pending_fs::get(db, path))
        .expect("load pending entry");
    assert!(pending.is_some());
}

#[test]
fn discard_tracked_add_rebinds_workspace_inode() {
    let (_dir, repo) = new_repo();
    let file = workspace_path(&repo, "notes/a.md");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, "hello").expect("write file");
    let repo_root = repo
        .local_repo_workspace_root(repo.local_repo_name())
        .expect("workspace root");
    let repo = Arc::new(repo);
    let vfs = Vfs::new(repo_root);
    scan::scan_projection_workspaces(&repo, &vfs).expect("scan initial");
    repo.stage_pending("notes/a.md").expect("stage file");
    repo.apply_external_changes().expect("apply external file");
    repo.commit_source_control_changes("initial")
        .expect("commit file");
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

#[test]
fn repo_discard_docless_added_on_tracked_path_fails_closed_before_delete() {
    let (_dir, repo) = new_repo();
    let repo = commit_initial_file(repo, "notes/a.md", "committed");
    seed_docless_added_pending(&repo, "notes/a.md", "replacement");

    let err = repo
        .discard_pending_target_in_local_repo(
            repo.local_repo_name(),
            &ScPathTarget::from_path("notes/a.md"),
        )
        .expect_err("docless added entry at tracked path must fail closed");

    assert_docless_added_guard(&err);
    assert_pending_and_file_preserved(&repo, "notes/a.md", "replacement");
}

#[test]
fn sync_discard_docless_added_on_tracked_path_fails_closed_before_delete() {
    let (_dir, repo) = new_repo();
    let repo = commit_initial_file(repo, "notes/a.md", "committed");
    seed_docless_added_pending(&repo, "notes/a.md", "replacement");
    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");

    let err = sync
        .discard_pending_target_in_local_repo(
            repo.local_repo_name(),
            &ScPathTarget::from_path("notes/a.md"),
        )
        .expect_err("docless added entry at tracked path must fail closed");

    assert_docless_added_guard(&err);
    assert_pending_and_file_preserved(&repo, "notes/a.md", "replacement");
}

#[cfg(unix)]
#[test]
fn discard_tracked_add_fails_closed_on_unstatable_workspace_path() {
    let (_dir, repo) = new_repo();
    let file = workspace_path(&repo, "notes/a.md");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, "hello").expect("write file");
    let repo_root = repo
        .local_repo_workspace_root(repo.local_repo_name())
        .expect("workspace root");
    let repo = Arc::new(repo);
    let vfs = Vfs::new(repo_root);
    scan::scan_projection_workspaces(&repo, &vfs).expect("scan initial");
    repo.stage_pending("notes/a.md").expect("stage file");
    repo.apply_external_changes().expect("apply external file");
    repo.commit_source_control_changes("initial")
        .expect("commit file");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");

    std::fs::remove_file(&file).expect("remove canonical");
    std::fs::write(workspace_path(&repo, "notes/b.md"), "hello").expect("write renamed file");
    let expected_pending = PendingFsEntry {
        path: "notes/b.md".into(),
        renamed_from: Some("notes/a.md".into()),
        doc_id: Some(doc_id),
        change_type: ChangeStatus::Added,
        content_hash: pending_fs::content_hash("hello"),
        detected_at: 2,
        has_conflict: false,
    };
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(db, &expected_pending)
    })
    .expect("seed tracked add");

    let notes_dir = workspace_path(&repo, "notes");
    let original = std::fs::metadata(&notes_dir)
        .expect("metadata")
        .permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&notes_dir, blocked).expect("chmod 000");

    let result = repo.discard_pending("notes/b.md");
    std::fs::set_permissions(&notes_dir, original).expect("restore perms");
    let err = result.expect_err("unstatable tracked add must fail closed");
    assert!(
        err.to_string()
            .contains("Failed to stat Projection Workspace ancestor while resolving"),
        "unexpected workspace containment diagnostic: {err:#}"
    );
    assert!(
        err.chain().any(|cause| {
            cause
                .downcast_ref::<std::io::Error>()
                .is_some_and(|io_err| io_err.kind() == std::io::ErrorKind::PermissionDenied)
        }),
        "workspace containment error must preserve PermissionDenied in its chain: {err:#}"
    );
    let pending = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            pending_fs::get(db, "notes/b.md")
        })
        .expect("load pending")
        .expect("pending entry preserved");
    assert_eq!(pending.path, expected_pending.path);
    assert_eq!(pending.renamed_from, expected_pending.renamed_from);
    assert_eq!(pending.doc_id, expected_pending.doc_id);
    assert_eq!(pending.change_type, expected_pending.change_type);
    assert_eq!(pending.content_hash, expected_pending.content_hash);
    assert_eq!(pending.detected_at, expected_pending.detected_at);
    assert_eq!(pending.has_conflict, expected_pending.has_conflict);
    assert_eq!(
        std::fs::read_to_string(workspace_path(&repo, "notes/b.md"))
            .expect("tracked add file preserved"),
        "hello"
    );
    assert!(!file.try_exists().expect("stat canonical path"));
}
