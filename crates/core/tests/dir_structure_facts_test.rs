use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEvent, StructureOp};
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, repo)
}

fn dir_ops(repo: &RepoManager, doc_id: DocId) -> Vec<StructureOp> {
    repo.get_local_ops(doc_id)
        .expect("load dir ops")
        .into_iter()
        .filter_map(|(_, entry)| match entry.event {
            LedgerEvent::Structure(op) => Some(op),
            LedgerEvent::Content(_) => None,
        })
        .collect()
}

#[test]
fn dir_create_and_rename_emit_structure_facts() {
    let (_dir, repo) = new_repo();
    let node_id = repo
        .apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes/sub", "test")
        .expect("create dir structure");
    let event_doc_id = DocId::from_u128(node_id.as_u128());
    let ops = dir_ops(&repo, event_doc_id);
    assert!(
        ops.iter()
            .any(|op| matches!(op, StructureOp::CreateDir { name, .. } if name == "notes"))
    );
    assert!(
        ops.iter()
            .any(|op| matches!(op, StructureOp::CreateDir { name, .. } if name == "sub"))
    );

    repo.apply_dir_rename_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/sub",
        "archive/sub-renamed",
        "test",
    )
    .expect("rename dir structure");
    let ops = dir_ops(&repo, event_doc_id);
    assert!(
        ops.iter()
            .any(|op| matches!(op, StructureOp::MoveNode { .. }))
    );
    assert!(ops.iter().any(
        |op| matches!(op, StructureOp::RenameNode { new_name, .. } if new_name == "sub-renamed")
    ));
}

#[test]
fn dir_delete_emits_delete_structure_fact() {
    let (_dir, repo) = new_repo();
    let node_id = repo
        .apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes/sub", "test")
        .expect("create dir structure");
    let event_doc_id = DocId::from_u128(node_id.as_u128());
    repo.apply_dir_delete_structure_in_local_repo(repo.local_repo_name(), "notes/sub", "test")
        .expect("delete dir structure");
    assert!(
        dir_ops(&repo, event_doc_id)
            .iter()
            .any(|op| matches!(op, StructureOp::DeleteNode { .. }))
    );
}
