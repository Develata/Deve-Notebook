use deve_core::config::SyncMode;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
use deve_core::models::{LedgerEntry, LedgerEvent, NodeId, Op, PeerId, RepoId, RepoType};
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

fn write_repo_info(db: &redb::Database, info: &RepoInfo) {
    let txn = db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(&0, bincode::serialize(info).expect("serialize").as_slice())
        .expect("write metadata");
    txn.commit().expect("commit");
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

    assert!(response.ops.len() >= 2);
    let entries = response
        .ops
        .iter()
        .map(|enc| repo_key.decrypt(enc).expect("decrypt snapshot entry"))
        .collect::<Vec<_>>();
    assert!(entries.iter().any(|entry| entry.doc_id == Some(doc_id)));
    assert!(entries.iter().any(|entry| {
        matches!(
            entry.event,
            LedgerEvent::Structure(deve_core::models::StructureOp::CreateFile {
                doc_id: file_doc_id,
                ..
            }) if file_doc_id == doc_id
        )
    }));
}

#[test]
fn local_repo_reads_fail_closed_on_stale_metadata_name_selector() {
    let (_dir, repo, extra_id, extra_name) = new_local_repos();
    let extra_db = repo.open_database(None, &extra_name).expect("extra db");
    write_repo_info(
        extra_db.db.as_ref(),
        &RepoInfo {
            uuid: extra_id,
            name: "legacy-wiki".into(),
            url: Some("urn:wiki".into()),
        },
    );

    let err = repo
        .get_repo_info_for(None, Some("legacy-wiki"))
        .expect_err("stale local alias must fail closed");
    assert!(err.to_string().contains("metadata name drifted to legacy-wiki"));

    let err = repo
        .run_on_local_repo("legacy-wiki", |_db| Ok::<_, anyhow::Error>(()))
        .expect_err("stale local alias must fail closed");
    assert!(err.to_string().contains("metadata name drifted to legacy-wiki"));
}

#[test]
fn workspace_resolution_keeps_execution_repo_stem_after_metadata_drift() {
    let (_dir, repo, extra_id, extra_name) = new_local_repos();
    let extra_db = repo.open_database(None, &extra_name).expect("extra db");
    write_repo_info(
        extra_db.db.as_ref(),
        &RepoInfo {
            uuid: extra_id,
            name: "legacy-wiki".into(),
            url: Some("urn:wiki".into()),
        },
    );

    let (repo_name, repo_id, repo_path) = repo
        .resolve_local_workspace_path("wiki/notes/extra.md")
        .expect("resolve workspace path")
        .expect("resolved repo scope");
    assert_eq!(repo_name, "wiki");
    assert_eq!(repo_id, extra_id);
    assert_eq!(repo_path, "notes/extra.md");
}

#[test]
fn execution_resolution_rejects_uuid_string_in_repo_name_slot() {
    let (_dir, repo, extra_id, _extra_name) = new_local_repos();
    let uuid_selector = extra_id.to_string();

    let err = repo
        .resolve_local_repo_name_for_execution(None, Some(&uuid_selector))
        .expect_err("uuid-shaped repo_name must fail closed");
    assert!(err.to_string().contains(&uuid_selector));

    assert!(
        repo.get_repo_info_for(None, Some(&uuid_selector))
            .expect("lookup repo info")
            .is_none()
    );
    assert!(
        repo.resolve_local_workspace_path(&format!("{uuid_selector}/notes/extra.md"))
            .expect("resolve workspace path")
            .is_none()
    );
}

#[test]
fn local_repo_id_lookup_fails_closed_when_secondary_metadata_is_unreadable() {
    let (_dir, repo, extra_id, extra_name) = new_local_repos();
    let extra_db = repo.open_database(None, &extra_name).expect("extra db");
    let txn = extra_db.db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(&0, [0_u8, 1, 2, 3].as_slice())
        .expect("write broken metadata");
    txn.commit().expect("commit broken metadata");

    let err = repo
        .find_local_repo_name_by_id(extra_id)
        .expect_err("broken secondary repo metadata must fail closed");
    assert!(
        err.to_string().contains("decode")
            || err.to_string().contains("deserialize")
            || err.to_string().contains("unexpected end")
    );
}
