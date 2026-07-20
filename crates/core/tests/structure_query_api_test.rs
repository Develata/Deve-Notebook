use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, NodeId, PeerId, RepoType, StructureOp};
use tempfile::TempDir;
use uuid::Uuid;

mod common;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) =
        common::init_cataloged_repo(&dir.path().join("ledger"), &dir.path().join("notes"))
            .expect("init cataloged repo");
    (dir, repo)
}

#[test]
fn repo_manager_reads_structure_ops_by_node_for_local_and_remote() {
    let (_dir, repo) = new_repo();
    let (local_node, _ops) = repo
        .apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes", "test")
        .expect("create local dir");
    assert_eq!(
        repo.get_local_structure_ops_in_local_repo(repo.local_repo_name(), local_node)
            .expect("load local structure ops")
            .len(),
        1
    );

    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let remote_node = NodeId::new();
    repo.append_remote_op(
        &peer_id,
        &repo_id,
        &LedgerEntry::new_structure(
            StructureOp::CreateDir {
                node_id: remote_node,
                parent_id: None,
                name: "remote".into(),
            },
            1,
            peer_id.clone(),
            1,
        ),
    )
    .expect("append remote structure op");

    assert_eq!(
        repo.get_structure_ops(&RepoType::Remote(peer_id, repo_id), remote_node)
            .expect("load remote structure ops")
            .len(),
        1
    );
}
