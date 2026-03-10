use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEvent, StructureOp};
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

fn seed_pending(repo: &RepoManager, entry: PendingFsEntry) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| pending_fs::upsert(db, &entry))
        .expect("seed pending entry");
}

fn structure_ops(repo: &RepoManager, doc_id: DocId) -> Vec<StructureOp> {
    repo.get_local_ops(doc_id)
        .expect("load ops")
        .into_iter()
        .filter_map(|(_, entry)| match entry.event {
            LedgerEvent::Structure(op) => Some(op),
            LedgerEvent::Content(_) => None,
        })
        .collect()
}

#[test]
fn commit_emits_create_and_rename_structure_facts() {
    let (dir, repo) = new_repo();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 1,
            has_conflict: false,
        },
    );
    repo.stage_pending("notes/a.md").expect("stage add");
    repo.commit_staged("initial").expect("commit add");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");
    assert!(
        structure_ops(&repo, doc_id)
            .iter()
            .any(|op| matches!(op, StructureOp::CreateFile { .. }))
    );

    write_workspace_file(&dir, "notes/b.md", "hello");
    std::fs::remove_file(dir.path().join("vault").join("default").join("notes/a.md"))
        .expect("remove old path");
    seed_pending(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Deleted,
            content_hash: String::new(),
            detected_at: 2,
            has_conflict: false,
        },
    );
    seed_pending(
        &repo,
        PendingFsEntry {
            path: "notes/b.md".into(),
            renamed_from: Some("notes/a.md".into()),
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 2,
            has_conflict: false,
        },
    );
    repo.stage_pending("notes/a.md").expect("stage delete");
    repo.stage_pending("notes/b.md").expect("stage rename add");
    repo.commit_staged("rename").expect("commit rename");
    assert!(structure_ops(&repo, doc_id).iter().any(|op| matches!(
        op,
        StructureOp::RenameNode { new_name, .. } if new_name == "b.md"
    )));
}
