use deve_core::ledger::{RepoManager, ops};
use deve_core::models::{LedgerEvent, NodeId, StructureOp};
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

fn dir_ops(repo: &RepoManager, node_id: NodeId) -> Vec<StructureOp> {
    dir_entries(repo, node_id)
        .into_iter()
        .filter_map(|(_, entry)| match entry.event {
            LedgerEvent::Structure(op) => Some(op),
            LedgerEvent::Content(_) | LedgerEvent::MergeAnchor(_) => None,
        })
        .collect()
}

fn dir_entries(repo: &RepoManager, node_id: NodeId) -> Vec<(u64, deve_core::models::LedgerEntry)> {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        ops::get_structure_ops_for_node_from_db(db, node_id)
    })
    .expect("load dir ops")
}

#[test]
fn dir_create_and_rename_emit_structure_facts() {
    let (_dir, repo) = new_repo();
    let (node_id, _ops) = repo
        .apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes/sub", "test")
        .expect("create dir structure");
    let ops = dir_ops(&repo, node_id);
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
    let ops = dir_ops(&repo, node_id);
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
    let (node_id, _ops) = repo
        .apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes/sub", "test")
        .expect("create dir structure");
    repo.apply_dir_delete_structure_in_local_repo(repo.local_repo_name(), "notes/sub", "test")
        .expect("delete dir structure");
    assert!(
        dir_ops(&repo, node_id)
            .iter()
            .any(|op| matches!(op, StructureOp::DeleteNode { .. }))
    );
}

#[test]
fn dir_structure_facts_preserve_monotonic_peer_wide_order() {
    let (_dir, repo) = new_repo();
    let (node_id, _ops) = repo
        .apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes/sub", "test")
        .expect("create dir structure");
    repo.apply_dir_rename_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/sub",
        "archive/sub-renamed",
        "test",
    )
    .expect("rename dir structure");
    repo.apply_dir_delete_structure_in_local_repo(
        repo.local_repo_name(),
        "archive/sub-renamed",
        "test",
    )
    .expect("delete dir structure");
    let seqs: Vec<_> = dir_entries(&repo, node_id)
        .into_iter()
        .map(|(_, entry)| entry.peer_seq.get())
        .collect();
    assert_eq!(seqs.len(), 4);
    assert!(seqs.windows(2).all(|pair| pair[0] < pair[1]));
}
