//! NET-010: Shadow 写隔离 — remote ops 仅影响 shadow repo，不污染 local。

use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{DocId, LedgerEntry, NodeId, Op, PeerId, RepoType, StructureOp};
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
fn remote_structure_op_does_not_appear_in_local_listing() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let repo_id = repo.get_repo_info().unwrap().unwrap().uuid;

    // Create a local file
    let (local_doc, _ops) = repo
        .apply_file_structure_in_local_repo(&name, "local.md", None, "test")
        .expect("create local file");

    // Append a remote structure op from a "malicious" peer
    let peer_id = PeerId::new("malicious");
    let remote_repo_id = Uuid::new_v4();
    let remote_doc = DocId::new();
    repo.append_remote_op(
        &peer_id,
        &remote_repo_id,
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::from_doc_id(remote_doc),
                doc_id: remote_doc,
                parent_id: None,
                name: "evil.md".into(),
            },
            1,
            peer_id.clone(),
            1,
        ),
    )
    .expect("append remote file");

    // Local listing must NOT contain the remote doc
    let local_docs = repo
        .list_docs(&RepoType::Local(repo_id))
        .expect("list local docs");
    assert_eq!(local_docs.len(), 1);
    assert_eq!(local_docs[0].0, local_doc);

    // Shadow listing must contain the remote doc
    let shadow_docs = repo
        .list_docs(&RepoType::Remote(peer_id, remote_repo_id))
        .expect("list shadow docs");
    assert_eq!(shadow_docs.len(), 1);
    assert_eq!(shadow_docs[0].0, remote_doc);
}

#[test]
fn remote_content_op_does_not_appear_in_local_doc_ops() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let peer_id = PeerId::new("remote-peer");
    let remote_repo_id = Uuid::new_v4();

    // Create local file with content
    let (local_doc, _ops) = repo
        .apply_file_structure_in_local_repo(&name, "mine.md", None, "test")
        .expect("create local file");
    let local_peer = repo.local_peer_id().clone();
    repo.append_generated_op_in_local_repo(&name, local_doc, local_peer.clone(), |seq| {
        LedgerEntry::new_content(
            local_doc,
            Op::Insert {
                pos: 0,
                content: "safe".into(),
            },
            1,
            local_peer.clone(),
            seq,
            None,
            None,
        )
    })
    .expect("local content op");

    // Append remote content op
    let remote_doc = DocId::new();
    repo.append_remote_op(
        &peer_id,
        &remote_repo_id,
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::from_doc_id(remote_doc),
                doc_id: remote_doc,
                parent_id: None,
                name: "remote.md".into(),
            },
            1,
            peer_id.clone(),
            1,
        ),
    )
    .expect("remote structure");
    repo.append_remote_op(
        &peer_id,
        &remote_repo_id,
        &LedgerEntry::new_content(
            remote_doc,
            Op::Insert {
                pos: 0,
                content: "injected".into(),
            },
            2,
            peer_id.clone(),
            2,
            None,
            None,
        ),
    )
    .expect("remote content");

    // Local ops should only have structure + content ops, no remote injection
    let local_ops = repo
        .get_local_ops_in_local_repo(&name, local_doc)
        .expect("get local ops");
    let content_ops: Vec<_> = local_ops
        .iter()
        .filter(|(_, e)| e.content_op().is_some())
        .collect();
    assert_eq!(content_ops.len(), 1);
    assert_eq!(
        content_ops[0].1.content_op(),
        Some(&Op::Insert {
            pos: 0,
            content: "safe".into(),
        })
    );
}

#[test]
fn remote_node_does_not_appear_in_local_node_listing() {
    let (_dir, repo) = new_repo();
    let repo_id = repo.get_repo_info().unwrap().unwrap().uuid;

    let peer_id = PeerId::new("attacker");
    let remote_repo_id = Uuid::new_v4();
    let remote_node = NodeId::new();
    repo.append_remote_op(
        &peer_id,
        &remote_repo_id,
        &LedgerEntry::new_structure(
            StructureOp::CreateDir {
                node_id: remote_node,
                parent_id: None,
                name: "hacked-folder".into(),
            },
            1,
            peer_id.clone(),
            1,
        ),
    )
    .expect("append remote dir");

    let local_nodes = repo
        .list_nodes(&RepoType::Local(repo_id))
        .expect("list local nodes");
    assert!(
        !local_nodes.iter().any(|n| n.1.path == "hacked-folder"),
        "remote dir must not leak into local listing"
    );
}
