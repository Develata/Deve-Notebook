use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{LedgerEntry, NodeId, Op, PeerId, RepoId, RepoType};
use deve_core::security::RepoKey;
use deve_core::sync::engine::SyncEngine;
use deve_core::sync::protocol::SyncSnapshotRequest;
use std::sync::Arc;
use tempfile::TempDir;

fn new_local_repos() -> (TempDir, RepoManager, RepoId, String) {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main =
        RepoManager::init(&ledger_dir, 10, Some("main"), Some("urn:main")).expect("init main repo");
    let extra = RepoManager::init(&ledger_dir, 10, Some("wiki"), Some("urn:wiki"))
        .expect("init extra repo");
    let extra_info = extra
        .get_repo_info()
        .expect("read extra repo info")
        .expect("extra repo info present");
    (dir, main, extra_info.uuid, extra_info.name)
}

fn seed_extra_doc(repo: &RepoManager, repo_name: &str) -> deve_core::models::DocId {
    let doc_id = repo
        .apply_file_structure_in_local_repo(repo_name, "notes/extra.md", None, "test")
        .expect("create extra file");
    repo.append_generated_op_in_local_repo(repo_name, doc_id, PeerId::new("local"), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "extra repo".into(),
            },
            1,
            PeerId::new("local"),
            seq,
            None,
            None,
        )
    })
    .expect("append extra content");
    doc_id
}

#[test]
fn local_repo_reads_route_by_repo_id() {
    let (_dir, repo, extra_id, extra_name) = new_local_repos();
    let doc_id = seed_extra_doc(&repo, &extra_name);
    let repo_type = RepoType::Local(extra_id);

    assert_eq!(
        repo.list_docs(&repo_type).expect("list docs"),
        vec![(doc_id, "notes/extra.md".to_string())]
    );
    assert_eq!(
        repo.get_structure_ops(&repo_type, NodeId::from_doc_id(doc_id))
            .expect("load structure ops")
            .len(),
        1
    );
    assert_eq!(
        repo.get_ops(&repo_type, doc_id)
            .expect("load doc ops")
            .len(),
        2
    );
    assert_eq!(
        repo.get_local_ops_in_range(&extra_id, 1, 8)
            .expect("load ranged ops")
            .len(),
        3
    );
}

#[test]
fn sync_snapshot_uses_requested_local_repo_id() {
    let (_dir, repo, extra_id, extra_name) = new_local_repos();
    let doc_id = seed_extra_doc(&repo, &extra_name);
    let repo_key = RepoKey::generate();
    let engine = SyncEngine::new(
        PeerId::new("local"),
        Arc::new(repo),
        SyncMode::Auto,
        Some(repo_key.clone()),
    );

    let response = engine
        .get_snapshot_for_sync(&SyncSnapshotRequest {
            peer_id: PeerId::new("local"),
            repo_id: extra_id,
        })
        .expect("build sync snapshot");

    assert_eq!(response.ops.len(), 1);
    let entry = repo_key
        .decrypt(&response.ops[0])
        .expect("decrypt snapshot entry");
    assert_eq!(entry.doc_id, Some(doc_id));
}
