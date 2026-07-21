use deve_core::config::SyncMode;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{LedgerEntry, LedgerEvent, NodeId, Op, RepoId, RepoType};
use deve_core::security::RepoKey;
use deve_core::sync::engine::SyncEngine;
use deve_core::sync::protocol::SyncSnapshotRequest;
use std::sync::{Arc, Barrier};
use tempfile::TempDir;

mod common;

fn new_local_repos() -> (TempDir, RepoManager, RepoId, String) {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("main-notes"), 10)
            .expect("init main repo");
    let wiki_id = common::add_cataloged_repo_with_depth(&main, &dir.path().join("wiki-notes"), 10)
        .expect("init wiki repo");
    (dir, main, wiki_id, wiki_id.to_string())
}

fn seed_extra_doc(repo: &RepoManager, repo_name: &str) -> deve_core::models::DocId {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo_name, "notes/extra.md", None, "test")
        .expect("create extra file");
    let peer = repo.local_peer_id().clone();
    repo.append_generated_op_in_local_repo(repo_name, doc_id, peer.clone(), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "extra repo".into(),
            },
            1,
            peer.clone(),
            seq,
            None,
            None,
        )
    })
    .expect("append extra content");
    doc_id
}

fn primary_repo_id(repo: &RepoManager, extra_id: RepoId) -> RepoId {
    repo.list_local_repo_names_for_execution()
        .expect("list durable local repos")
        .into_iter()
        .map(|name| name.parse::<RepoId>().expect("RepoId execution name"))
        .find(|repo_id| *repo_id != extra_id)
        .expect("primary RepoId")
}

#[test]
fn concurrent_catalog_refreshes_share_the_process_cut() {
    let (_dir, repo, extra_id, _extra_name) = new_local_repos();
    let primary_id = primary_repo_id(&repo, extra_id);
    let mut expected = vec![primary_id.to_string(), extra_id.to_string()];
    expected.sort();
    let repo = Arc::new(repo);
    let barrier = Arc::new(Barrier::new(5));
    let mut workers = Vec::new();

    for _ in 0..4 {
        let repo = repo.clone();
        let barrier = barrier.clone();
        let expected = expected.clone();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            for _ in 0..32 {
                let mut names = repo
                    .list_local_repo_names_for_execution()
                    .expect("concurrent catalog refresh");
                names.sort();
                assert_eq!(names, expected);
            }
        }));
    }

    barrier.wait();
    for worker in workers {
        worker.join().expect("catalog refresh worker");
    }
}

#[test]
fn local_repo_reads_route_by_repo_id() {
    let (_dir, repo, extra_id, extra_name) = new_local_repos();
    let main_id = primary_repo_id(&repo, extra_id);
    let main_doc_id = seed_extra_doc(&repo, &main_id.to_string());
    let doc_id = seed_extra_doc(&repo, &extra_name);
    let repo_type = RepoType::Local(extra_id);
    let main_type = RepoType::Local(main_id);

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
        repo.get_local_ops_in_range(
            &extra_id,
            repo.local_peer_id(),
            1_u64.into(),
            repo.get_local_peer_waterline(&extra_id)
                .expect("peer waterline"),
        )
        .expect("load ranged ops")
        .len(),
        3
    );
    assert_eq!(
        repo.list_docs(&main_type).expect("list primary docs"),
        vec![(main_doc_id, "notes/extra.md".to_string())]
    );
    assert!(
        repo.list_docs(&repo_type)
            .expect("re-list secondary docs")
            .iter()
            .all(|(listed, _)| *listed != main_doc_id),
        "secondary reads must not leak primary facts"
    );
    assert!(
        repo.list_docs(&main_type)
            .expect("re-list primary docs")
            .iter()
            .all(|(listed, _)| *listed != doc_id),
        "primary reads must not leak secondary facts"
    );
}

#[test]
fn sync_snapshot_uses_requested_local_repo_id() {
    let (_dir, repo, extra_id, extra_name) = new_local_repos();
    let main_id = primary_repo_id(&repo, extra_id);
    let main_doc_id = seed_extra_doc(&repo, &main_id.to_string());
    let doc_id = seed_extra_doc(&repo, &extra_name);
    let repo_key = RepoKey::generate();
    let local_peer = repo.local_peer_id().clone();
    let engine = SyncEngine::new(
        local_peer.clone(),
        Arc::new(repo),
        SyncMode::Auto,
        Some(repo_key.clone()),
    );

    let response = engine
        .get_snapshot_for_sync(&SyncSnapshotRequest {
            peer_id: local_peer,
            repo_id: extra_id,
            reason: None,
        })
        .expect("build sync snapshot");

    assert!(response.ops.len() >= 2);
    let entries = response
        .ops
        .iter()
        .map(|enc| repo_key.decrypt(enc).expect("decrypt snapshot entry"))
        .collect::<Vec<_>>();
    assert!(entries.iter().any(|entry| entry.doc_id == Some(doc_id)));
    assert!(
        entries
            .iter()
            .all(|entry| entry.doc_id != Some(main_doc_id)),
        "secondary snapshot must not contain primary content facts"
    );
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
    let (dir, mut repo, extra_id, _extra_name) = new_local_repos();
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locators");
    let extra_db = repo
        .lease_local_authority(extra_id)
        .expect("extra authority lease");
    common::write_repo_metadata(
        extra_db.db(),
        &RepoInfo {
            uuid: extra_id,
            name: "legacy-wiki".into(),
            url: Some("urn:wiki".into()),
        },
    );

    let err = repo
        .get_repo_info_for(None, Some("legacy-wiki"))
        .expect_err("stale local alias must fail closed");
    assert!(
        err.to_string()
            .contains("metadata name drifted to legacy-wiki")
    );

    let err = repo
        .run_on_local_repo("legacy-wiki", |_db| Ok::<_, anyhow::Error>(()))
        .expect_err("stale local alias must fail closed");
    assert!(
        err.to_string()
            .contains("metadata name drifted to legacy-wiki")
    );
}

#[test]
fn workspace_resolution_fails_closed_after_locator_alias_drift() {
    let (dir, mut repo, extra_id, _extra_name) = new_local_repos();
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locators");
    let extra_db = repo
        .lease_local_authority(extra_id)
        .expect("extra authority lease");
    common::write_repo_metadata(
        extra_db.db(),
        &RepoInfo {
            uuid: extra_id,
            name: "legacy-wiki".into(),
            url: Some("urn:wiki".into()),
        },
    );

    let err = repo
        .resolve_local_workspace_path("wiki/notes/extra.md")
        .expect_err("locator alias drift must fail closed");
    assert!(
        err.to_string()
            .contains("metadata name drifted to legacy-wiki")
    );
}

#[test]
fn execution_resolution_accepts_repo_id_physical_stem() {
    let (_dir, repo, extra_id, _extra_name) = new_local_repos();
    let uuid_selector = extra_id.to_string();

    assert_eq!(
        repo.resolve_local_repo_name_for_execution(None, Some(&uuid_selector))
            .expect("RepoId execution selector"),
        uuid_selector
    );

    assert_eq!(
        repo.get_repo_info_for(None, Some(&uuid_selector))
            .expect("lookup repo info")
            .expect("repo info")
            .uuid,
        extra_id
    );
    let (_, repo_id, repo_path) = repo
        .resolve_local_workspace_path(&format!("{uuid_selector}/notes/extra.md"))
        .expect("resolve workspace path")
        .expect("resolved RepoId execution prefix");
    assert_eq!(repo_id, extra_id);
    assert_eq!(repo_path, "notes/extra.md");
}

#[test]
fn local_repo_id_lookup_fails_closed_when_secondary_metadata_is_unreadable() {
    let (_dir, repo, extra_id, _extra_name) = new_local_repos();
    let extra_db = repo
        .lease_local_authority(extra_id)
        .expect("extra authority lease");
    common::poison_repo_metadata_invalid_codec(extra_db.db());

    let err = repo
        .find_local_repo_name_by_id(extra_id)
        .expect_err("broken secondary repo metadata must fail closed");
    assert!(
        err.to_string().contains("decode")
            || err.to_string().contains("deserialize")
            || err.to_string().contains("deserialization")
            || err.to_string().contains("postcard")
            || err.to_string().contains("unexpected end")
    );
}

#[test]
fn workspace_resolution_fails_closed_when_secondary_repo_info_is_missing() {
    let (_dir, repo, extra_id, _extra_name) = new_local_repos();
    let extra_db = repo
        .lease_local_authority(extra_id)
        .expect("extra authority lease");
    common::delete_repo_metadata(extra_db.db());

    let err = repo
        .resolve_local_workspace_path("wiki/notes/extra.md")
        .expect_err("missing repo info must fail closed");
    assert!(err.to_string().contains("while resolving workspace path"));
    assert!(err.to_string().contains(&extra_id.to_string()));
}

#[test]
fn exact_repo_id_execution_fails_closed_when_metadata_is_missing() {
    let (_dir, repo, extra_id, _extra_name) = new_local_repos();
    let extra_db = repo
        .lease_local_authority(extra_id)
        .expect("extra authority lease");
    common::delete_repo_metadata(extra_db.db());

    let err = repo
        .run_on_local_repo(&extra_id.to_string(), |_db| Ok::<_, anyhow::Error>(()))
        .expect_err("exact RepoId must not bypass missing RepoInfo");

    assert!(err.to_string().contains("repository metadata missing"));
}

#[test]
fn exact_repo_id_execution_fails_closed_when_metadata_repo_id_drifts() {
    let (_dir, repo, extra_id, extra_name) = new_local_repos();
    let extra_db = repo
        .lease_local_authority(extra_id)
        .expect("extra authority lease");
    let drifted_id = uuid::Uuid::new_v4();
    common::write_repo_metadata(
        extra_db.db(),
        &RepoInfo {
            uuid: drifted_id,
            name: extra_name,
            url: Some("urn:wiki".into()),
        },
    );

    let err = repo
        .run_on_local_repo(&extra_id.to_string(), |_db| Ok::<_, anyhow::Error>(()))
        .expect_err("exact RepoId must match durable RepoInfo");

    assert!(
        err.to_string()
            .contains("physical RepoId does not match metadata RepoId")
    );
    assert!(err.to_string().contains(&drifted_id.to_string()));
}
