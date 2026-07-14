#![cfg(unix)]

use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op};
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
        repo.local_peer_id().clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                repo.local_peer_id().clone(),
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

fn assert_workspace_ancestor_permission_denied(err: &anyhow::Error) {
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
    let target = blocked.join("a.md");
    std::fs::create_dir_all(&blocked).expect("create blocked dir");
    let original = block_dir(&blocked);

    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    let result = sync.materialize_local_repo("default");
    std::fs::set_permissions(&blocked, original).expect("restore perms");
    let err = result.expect_err("unstatable workspace path must fail closed");
    assert_workspace_ancestor_permission_denied(&err);
    assert!(
        !target.try_exists().expect("stat materialize target"),
        "failed materialization must not leave a workspace projection"
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
    let op_count_before = repo
        .get_local_ops_in_local_repo(repo.local_repo_name(), doc_id)
        .expect("load ledger ops before reconcile")
        .len();
    std::fs::create_dir_all(&blocked).expect("create blocked dir");
    let original = block_dir(&blocked);

    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    let result = sync.reconcile_doc_in_local_repo("default", doc_id);
    std::fs::set_permissions(&blocked, original).expect("restore perms");
    let err = result.expect_err("unstatable reconcile path must fail closed");
    assert_workspace_ancestor_permission_denied(&err);
    assert_eq!(
        repo.get_local_ops_in_local_repo(repo.local_repo_name(), doc_id)
            .expect("load ledger ops after reconcile")
            .len(),
        op_count_before,
        "failed reconciliation must not append ledger facts"
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
    let inode_docids_before = repo
        .run_on_local_repo(
            repo.local_repo_name(),
            deve_core::ledger::inode_index::list_docids,
        )
        .expect("load inode index before bind");
    std::fs::create_dir_all(&blocked).expect("create blocked dir");
    let original = block_dir(&blocked);

    let result = repo.bind_workspace_inode_in_local_repo("default", "notes/a.md", doc_id);
    std::fs::set_permissions(&blocked, original).expect("restore perms");
    let err = result.expect_err("unstatable workspace path must fail closed");
    assert_workspace_ancestor_permission_denied(&err);
    assert_eq!(
        repo.run_on_local_repo(
            repo.local_repo_name(),
            deve_core::ledger::inode_index::list_docids
        )
        .expect("load inode index after bind"),
        inode_docids_before,
        "failed inode binding must not change the inode index"
    );
}
