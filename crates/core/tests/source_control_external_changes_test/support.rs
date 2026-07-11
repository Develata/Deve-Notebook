use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op};
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeDomain, ChangeStatus};
use tempfile::{TempDir, tempdir};

pub(crate) fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, repo)
}

pub(crate) fn write_workspace_file(repo: &RepoManager, path: &str, content: &str) {
    let abs = repo
        .local_repo_workspace_path("default", path)
        .expect("workspace path");
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

pub(crate) fn workspace_path(repo: &RepoManager, path: &str) -> std::path::PathBuf {
    repo.local_repo_workspace_path("default", path)
        .expect("workspace path")
}

pub(crate) fn seed_pending(repo: &RepoManager, path: &str, status: ChangeStatus, content: &str) {
    let doc_id = repo
        .get_tracked_docid_in_local_repo(repo.local_repo_name(), path)
        .expect("resolve tracked doc id for pending seed");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: None,
                doc_id,
                change_type: status,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending entry");
}

pub(crate) fn seed_initial_commit(repo: &RepoManager) -> DocId {
    write_workspace_file(repo, "notes/a.md", "hello");
    seed_pending(repo, "notes/a.md", ChangeStatus::Added, "hello");
    repo.stage_pending("notes/a.md").expect("stage initial");
    repo.apply_external_changes()
        .expect("apply initial external change");
    repo.commit_source_control_changes("initial")
        .expect("initial commit");
    repo.get_tracked_docid_in_local_repo(repo.local_repo_name(), "notes/a.md")
        .expect("doc id lookup")
        .expect("tracked doc id")
}

pub(crate) fn append_confirmed_ledger_edit(repo: &RepoManager, doc_id: DocId) {
    let peer_id = repo.local_peer_id().clone();
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        peer_id.clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 5,
                    content: " world".into(),
                },
                1000,
                peer_id.clone(),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append editor ledger op");
}

pub(crate) fn ledger_head(repo: &RepoManager) -> u64 {
    repo.run_on_local_repo(
        repo.local_repo_name(),
        deve_core::ledger::range::get_max_seq,
    )
    .expect("ledger head")
}

pub(crate) fn assert_single_pending_external_change(
    repo: &RepoManager,
    path: &str,
    doc_id: Option<DocId>,
    status: ChangeStatus,
    before_head: u64,
) {
    let pending = repo.list_pending_fs().expect("pending external changes");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, path);
    assert_eq!(pending[0].doc_id, doc_id);
    assert_eq!(pending[0].status, status);
    assert_eq!(pending[0].domain, ChangeDomain::WorkingDirectory);
    assert_eq!(ledger_head(repo), before_head);
    assert!(
        repo.list_confirmed_ledger_changes()
            .expect("confirmed remains clean")
            .is_empty()
    );
    assert!(repo.list_staged().expect("staged remains clean").is_empty());
}
