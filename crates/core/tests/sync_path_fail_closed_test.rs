#![cfg(unix)]

use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::sync::SyncManager;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tempfile::TempDir;

fn new_repo() -> (TempDir, Arc<RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, Arc::new(repo))
}

fn seed_file(repo: &RepoManager, doc_path: &str, content: &str) -> deve_core::models::DocId {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), doc_path, None, "test")
        .expect("create file");
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append content");
    doc_id
}

#[cfg(unix)]
fn block_dir(path: &std::path::Path) -> std::fs::Permissions {
    let original = std::fs::metadata(path).expect("metadata").permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(path, perms).expect("chmod 000");
    original
}

#[cfg(unix)]
#[test]
fn materialize_local_repo_fails_closed_when_workspace_path_is_unstatable() {
    let (_dir, repo) = new_repo();
    seed_file(repo.as_ref(), "notes/a.md", "ledger");
    repo.ensure_local_repo_workspace_identity("default")
        .expect("identity marker");
    let blocked = repo
        .local_repo_workspace_path("default", "notes")
        .expect("workspace path");
    std::fs::create_dir_all(&blocked).expect("create blocked dir");
    let original = block_dir(&blocked);

    let sync = SyncManager::new_checked(repo).expect("sync manager");
    let err = sync
        .materialize_local_repo("default")
        .expect_err("unstatable workspace path must fail closed");

    std::fs::set_permissions(&blocked, original).expect("restore perms");
    let err_text = err.to_string();
    assert!(
        err_text.contains("Failed to stat workspace path while materializing projection")
            || err_text.contains("Permission denied"),
        "{err_text}"
    );
}

#[cfg(unix)]
#[test]
fn reconcile_doc_in_local_repo_fails_closed_when_workspace_path_is_unstatable() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_file(repo.as_ref(), "notes/a.md", "ledger");
    let blocked = repo
        .local_repo_workspace_path("default", "notes")
        .expect("workspace path");
    std::fs::create_dir_all(&blocked).expect("create blocked dir");
    let original = block_dir(&blocked);

    let sync = SyncManager::new_checked(repo).expect("sync manager");
    let err = sync
        .reconcile_doc_in_local_repo("default", doc_id)
        .expect_err("unstatable reconcile path must fail closed");

    std::fs::set_permissions(&blocked, original).expect("restore perms");
    assert!(
        err.to_string()
            .contains("Failed to stat workspace document path while reconciling")
            || err.to_string().contains("Permission denied")
    );
}

#[cfg(unix)]
#[test]
fn bind_workspace_inode_fails_closed_when_workspace_path_is_unstatable() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_file(repo.as_ref(), "notes/a.md", "ledger");
    let blocked = repo
        .local_repo_workspace_path("default", "notes")
        .expect("workspace path");
    std::fs::create_dir_all(&blocked).expect("create blocked dir");
    let original = block_dir(&blocked);

    let err = repo
        .bind_workspace_inode_in_local_repo("default", "notes/a.md", doc_id)
        .expect_err("unstatable workspace path must fail closed");

    std::fs::set_permissions(&blocked, original).expect("restore perms");
    assert!(
        err.to_string()
            .contains("Failed to stat workspace path while binding inode")
            || err.to_string().contains("Permission denied")
    );
}
