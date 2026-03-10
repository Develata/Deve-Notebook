use deve_core::ledger::{RepoManager, ops};
use deve_core::models::{LedgerEvent, NodeId, StructureOp};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, repo)
}

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

#[test]
fn delete_commit_emits_delete_structure_fact() {
    let (dir, repo) = new_repo();
    write_workspace_file(&dir, "notes/a.md", "hello");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed add");
    repo.stage_pending("notes/a.md").expect("stage add");
    repo.commit_staged("initial").expect("commit add");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");
    std::fs::remove_file(dir.path().join("vault").join("default").join("notes/a.md"))
        .expect("remove file");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Deleted,
                content_hash: String::new(),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed delete");
    repo.stage_pending("notes/a.md").expect("stage delete");
    repo.commit_staged("delete").expect("commit delete");
    let facts = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            ops::get_structure_ops_for_node_from_db(db, NodeId::from_doc_id(doc_id))
        })
        .expect("load ops")
        .into_iter()
        .filter_map(|(_, entry)| match entry.event {
            LedgerEvent::Structure(op) => Some(op),
            LedgerEvent::Content(_) => None,
        })
        .collect::<Vec<_>>();
    assert!(
        facts
            .iter()
            .any(|op| matches!(op, StructureOp::DeleteNode { .. }))
    );
}
