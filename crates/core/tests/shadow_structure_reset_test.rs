use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{DocId, LedgerEntry, NodeId, PeerId, RepoType, StructureOp};
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

#[test]
fn append_remote_structure_updates_shadow_projection() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let dir_id = NodeId::new();
    let doc_id = DocId::new();

    repo.append_remote_op(
        &peer_id,
        &repo_id,
        &LedgerEntry::new_structure(
            StructureOp::CreateDir {
                node_id: dir_id,
                parent_id: None,
                name: "notes".into(),
            },
            1,
            peer_id.clone(),
            1,
        ),
    )
    .expect("append remote dir");
    repo.append_remote_op(
        &peer_id,
        &repo_id,
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::from_doc_id(doc_id),
                doc_id,
                parent_id: Some(dir_id),
                name: "remote.md".into(),
            },
            2,
            peer_id.clone(),
            2,
        ),
    )
    .expect("append remote file");

    let repo_type = RepoType::Remote(peer_id, repo_id);
    assert_eq!(
        repo.list_docs(&repo_type).expect("list shadow docs"),
        vec![(doc_id, "notes/remote.md".to_string())]
    );
    assert_eq!(
        repo.list_nodes(&repo_type)
            .expect("list shadow nodes")
            .len(),
        2
    );
}

#[test]
fn remote_repo_info_falls_back_to_repo_id_when_metadata_missing() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();

    repo.ensure_shadow_db(&peer_id, &repo_id)
        .expect("ensure shadow db");

    let info = repo
        .get_repo_info_for(Some(&peer_id), Some(&repo_id.to_string()))
        .expect("read remote repo info")
        .expect("fallback repo info");
    assert_eq!(info.uuid, repo_id);
    assert_eq!(info.name, repo_id.to_string());
    assert_eq!(info.url, None);
}
