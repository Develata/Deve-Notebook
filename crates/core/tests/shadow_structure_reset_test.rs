use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::Op;
use deve_core::models::{DocId, LedgerEntry, NodeId, PeerId, RepoType, StructureOp};
use deve_core::security::RepoKey;
use deve_core::sync::engine::SyncEngine;
use deve_core::sync::protocol::SyncSnapshotRequest;
use tempfile::TempDir;
use uuid::Uuid;

mod common;

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
fn remote_repo_info_fails_closed_when_metadata_missing() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    let repo_id = Uuid::new_v4();

    common::seed_shadow_without_metadata_row(&repo, &peer_id, repo_id);

    let err = repo
        .get_repo_info_for(Some(&peer_id), Some(&repo_id.to_string()))
        .expect_err("metadata-less remote repo info lookup must fail closed");
    assert!(
        err.to_string()
            .contains(format!("Broken shadow repo {} for peer {}", repo_id, peer_id).as_str())
    );
}

#[test]
fn apply_remote_snapshot_extends_a_matching_confirmed_prefix() {
    // Source and receiver live in isolated ledgers (production names the ledger
    // dir "ledger"); they intentionally share one RepoId so the receiver can
    // store the source's snapshot under its own shadow scope.
    let source_dir = tempfile::tempdir().expect("source tempdir");
    let receiver_dir = tempfile::tempdir().expect("receiver tempdir");
    let repo_id = uuid::Uuid::new_v4();
    let source_repo = common::init_cataloged_repo_with_id(
        &source_dir.path().join("ledger"),
        &source_dir.path().join("notes"),
        repo_id,
        "urn:source",
    )
    .expect("source repo");
    let receiver_repo = common::init_cataloged_repo_with_id(
        &receiver_dir.path().join("ledger"),
        &receiver_dir.path().join("notes"),
        repo_id,
        "urn:receiver",
    )
    .expect("receiver repo");
    let repo_key = RepoKey::generate();
    let source_peer = source_repo.local_peer_id().clone();
    let source_engine = SyncEngine::new(
        source_peer.clone(),
        std::sync::Arc::new(source_repo),
        SyncMode::Auto,
        Some(repo_key.clone()),
    );
    let mut receiver_engine = SyncEngine::new(
        receiver_repo.local_peer_id().clone(),
        std::sync::Arc::new(receiver_repo),
        SyncMode::Auto,
        Some(repo_key),
    );
    let local_name = source_engine.repo.local_repo_name().to_string();
    let (live_doc, _ops) = source_engine
        .repo
        .apply_file_structure_in_local_repo(&local_name, "notes/live.md", None, "test")
        .expect("create local file");
    let initial = source_engine
        .get_snapshot_for_sync(&SyncSnapshotRequest {
            peer_id: source_peer.clone(),
            repo_id,
            reason: None,
        })
        .expect("build initial snapshot");
    receiver_engine
        .apply_remote_snapshot(initial)
        .expect("apply initial snapshot");
    let writer = source_engine
        .repo
        .local_fact_writer(deve_core::models::FactActor::new("test").unwrap());
    writer
        .append_content_in_local_repo(
            &local_name,
            live_doc,
            Op::Insert {
                pos: 0,
                content: "fresh".into(),
            },
            1,
        )
        .expect("append local content");

    let response = source_engine
        .get_snapshot_for_sync(&SyncSnapshotRequest {
            peer_id: source_peer.clone(),
            repo_id,
            reason: None,
        })
        .expect("build snapshot");
    receiver_engine
        .apply_remote_snapshot(response)
        .expect("apply snapshot");

    let shadow = RepoType::Remote(source_peer, repo_id);
    assert_eq!(
        receiver_engine
            .repo
            .list_docs(&shadow)
            .expect("list shadow docs"),
        vec![(live_doc, "notes/live.md".into())]
    );
}
