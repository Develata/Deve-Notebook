use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::schema::REPO_METADATA;
use deve_core::models::Op;
use deve_core::models::{DocId, LedgerEntry, NodeId, PeerId, RepoType, StructureOp};
use deve_core::security::RepoKey;
use deve_core::sync::engine::SyncEngine;
use deve_core::sync::protocol::SyncSnapshotRequest;
use redb::Database;
use tempfile::TempDir;
use uuid::Uuid;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    (dir, repo)
}

fn create_legacy_shadow_without_metadata(repo: &RepoManager, peer_id: &PeerId, repo_id: Uuid) {
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let db = Database::create(peer_dir.join(format!("{repo_id}.redb"))).expect("legacy shadow");
    let txn = db.begin_write().expect("write txn");
    let _ = txn.open_table(REPO_METADATA).expect("repo metadata");
    txn.commit().expect("commit legacy shadow");
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

    create_legacy_shadow_without_metadata(&repo, &peer_id, repo_id);

    let err = repo
        .get_repo_info_for(Some(&peer_id), Some(&repo_id.to_string()))
        .expect_err("metadata-less remote repo info lookup must fail closed");
    assert!(
        err.to_string()
            .contains(format!("Broken shadow repo {} for peer {}", repo_id, peer_id).as_str())
    );
}

#[test]
fn apply_remote_snapshot_replaces_stale_shadow_repo_contents() {
    let (_dir, repo) = new_repo();
    let repo_key = RepoKey::generate();
    let mut engine = SyncEngine::new(
        PeerId::new("local"),
        std::sync::Arc::new(repo),
        SyncMode::Auto,
        Some(repo_key.clone()),
    );
    let repo_id = engine
        .repo
        .get_repo_info()
        .expect("repo info")
        .expect("present")
        .uuid;
    let local_name = engine.repo.local_repo_name().to_string();
    let live_doc = engine
        .repo
        .apply_file_structure_in_local_repo(&local_name, "notes/live.md", None, "test")
        .expect("create local file");
    engine
        .repo
        .append_generated_op_in_local_repo(&local_name, live_doc, PeerId::new("local"), |seq| {
            LedgerEntry::new_content(
                live_doc,
                Op::Insert {
                    pos: 0,
                    content: "fresh".into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        })
        .expect("append local content");

    let stale_doc = DocId::new();
    engine
        .repo
        .append_remote_op(
            &PeerId::new("local"),
            &repo_id,
            &LedgerEntry::new_structure(
                StructureOp::CreateFile {
                    node_id: NodeId::from_doc_id(stale_doc),
                    doc_id: stale_doc,
                    parent_id: None,
                    name: "stale.md".into(),
                },
                1,
                PeerId::new("local"),
                1,
            ),
        )
        .expect("append stale shadow file");

    let response = engine
        .get_snapshot_for_sync(&SyncSnapshotRequest {
            peer_id: PeerId::new("local"),
            repo_id,
        })
        .expect("build snapshot");
    engine
        .apply_remote_snapshot(response)
        .expect("apply snapshot");

    let shadow = RepoType::Remote(PeerId::new("local"), repo_id);
    assert_eq!(
        engine.repo.list_docs(&shadow).expect("list shadow docs"),
        vec![(live_doc, "notes/live.md".into())]
    );
}
