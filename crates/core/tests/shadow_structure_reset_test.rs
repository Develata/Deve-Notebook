use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, NodeId, PeerId, StructureOp};
use tempfile::TempDir;
use uuid::Uuid;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    (dir, repo)
}

#[test]
fn reset_shadow_node_clears_structure_index() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let node_id = NodeId::new();
    let entry = LedgerEntry::new_structure(
        StructureOp::CreateDir {
            node_id,
            parent_id: None,
            name: "notes".into(),
        },
        1,
        peer_id.clone(),
        1,
    );

    repo.append_remote_op(&peer_id, &repo_id, &entry)
        .expect("append remote structure op");
    assert_eq!(
        repo.get_shadow_structure_ops(&peer_id, &repo_id, node_id)
            .expect("load structure ops")
            .len(),
        1
    );

    repo.reset_shadow_node(&peer_id, &repo_id, &node_id)
        .expect("reset shadow node");
    assert!(
        repo.get_shadow_structure_ops(&peer_id, &repo_id, node_id)
            .expect("reload structure ops")
            .is_empty()
    );
}
