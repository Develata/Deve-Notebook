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

fn timestamp(seq: u64) -> i64 {
    i64::try_from(seq).expect("test seq fits i64")
}

fn create_dir(peer_id: &PeerId, node_id: NodeId, name: &str, seq: u64) -> LedgerEntry {
    LedgerEntry::new_structure(
        StructureOp::CreateDir {
            node_id,
            parent_id: None,
            name: name.into(),
        },
        timestamp(seq),
        peer_id.clone(),
        seq,
    )
}

fn create_file(
    peer_id: &PeerId,
    doc_id: DocId,
    parent_id: Option<NodeId>,
    name: &str,
    seq: u64,
) -> LedgerEntry {
    LedgerEntry::new_structure(
        StructureOp::CreateFile {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id,
            parent_id,
            name: name.into(),
        },
        timestamp(seq),
        peer_id.clone(),
        seq,
    )
}

fn rename_node(
    peer_id: &PeerId,
    node_id: NodeId,
    doc_id: Option<DocId>,
    new_name: &str,
    seq: u64,
) -> LedgerEntry {
    LedgerEntry::new_structure(
        StructureOp::RenameNode {
            node_id,
            doc_id,
            new_name: new_name.into(),
        },
        timestamp(seq),
        peer_id.clone(),
        seq,
    )
}

#[test]
fn append_remote_structure_batch_updates_shadow_projection() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let dir_id = NodeId::new();
    let doc_id = DocId::new();
    let entries = vec![
        create_dir(&peer_id, dir_id, "notes", 1),
        create_file(&peer_id, doc_id, Some(dir_id), "remote.md", 2),
    ];

    assert_eq!(
        repo.append_remote_ops(&peer_id, &repo_id, &entries)
            .expect("append remote batch"),
        2
    );

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
fn failed_remote_structure_batch_rolls_back_projection_and_ledger() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let first_doc = DocId::new();
    let second_doc = DocId::new();
    let entries = vec![
        create_file(&peer_id, first_doc, None, "dup.md", 1),
        create_file(&peer_id, second_doc, None, "dup.md", 2),
    ];

    let err = repo
        .append_remote_ops(&peer_id, &repo_id, &entries)
        .expect_err("duplicate path must fail projection");
    assert!(err.to_string().contains("structure path already"));
    assert_eq!(repo.get_shadow_max_seq(&peer_id, &repo_id).unwrap(), 0);
    assert!(
        repo.list_docs(&RepoType::Remote(peer_id, repo_id))
            .expect("list shadow docs")
            .is_empty()
    );
}

#[test]
fn create_dir_cannot_replace_existing_file_path() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let doc_id = DocId::new();
    let entries = vec![
        create_file(&peer_id, doc_id, None, "same", 1),
        create_dir(&peer_id, NodeId::new(), "same", 2),
    ];

    let err = repo
        .append_remote_ops(&peer_id, &repo_id, &entries)
        .expect_err("dir must not replace file path");
    assert!(err.to_string().contains("structure path already"));
    assert_eq!(repo.get_shadow_max_seq(&peer_id, &repo_id).unwrap(), 0);
    assert!(
        repo.list_docs(&RepoType::Remote(peer_id, repo_id))
            .expect("list shadow docs")
            .is_empty()
    );
}

#[test]
fn remote_rename_cannot_replace_existing_dir_path() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let dir_id = NodeId::new();
    let doc_id = DocId::new();
    repo.append_remote_ops(
        &peer_id,
        &repo_id,
        &[
            create_dir(&peer_id, dir_id, "taken", 1),
            create_file(&peer_id, doc_id, None, "free.md", 2),
        ],
    )
    .expect("seed tree");

    let err = repo
        .append_remote_op(
            &peer_id,
            &repo_id,
            &rename_node(
                &peer_id,
                NodeId::from_doc_id(doc_id),
                Some(doc_id),
                "taken",
                3,
            ),
        )
        .expect_err("rename must not replace dir path");
    assert!(err.to_string().contains("structure target path already"));
    assert_eq!(repo.get_shadow_max_seq(&peer_id, &repo_id).unwrap(), 2);
}

#[test]
fn remote_dir_rename_updates_shadow_doc_paths() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();
    let dir_id = NodeId::new();
    let doc_id = DocId::new();
    repo.append_remote_ops(
        &peer_id,
        &repo_id,
        &[
            create_dir(&peer_id, dir_id, "notes", 1),
            create_file(&peer_id, doc_id, Some(dir_id), "remote.md", 2),
        ],
    )
    .expect("seed remote tree");
    repo.append_remote_op(
        &peer_id,
        &repo_id,
        &rename_node(&peer_id, dir_id, None, "archive", 3),
    )
    .expect("rename remote dir");

    assert_eq!(
        repo.list_docs(&RepoType::Remote(peer_id, repo_id))
            .expect("list shadow docs"),
        vec![(doc_id, "archive/remote.md".to_string())]
    );
}
