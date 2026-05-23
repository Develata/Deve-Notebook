use deve_core::ledger::schema::NODEID_TO_META;
use deve_core::models::{LedgerEntry, NodeId, Op, PeerId};
use deve_core::sync::SyncManager;
use tempfile::TempDir;

fn new_repo() -> (TempDir, std::sync::Arc<deve_core::ledger::RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = deve_core::ledger::RepoManager::init(dir.path().join("ledger"), 10, None, None)
        .expect("init repo");
    repo.set_projection_base_for_all_local_repos(dir.path().join("vault"));
    (dir, std::sync::Arc::new(repo))
}

#[test]
fn persist_doc_prefers_node_projection_path() {
    let (dir, repo) = new_repo();
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), "notes/a.md", None, "test")
        .expect("create file structure");
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append content");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        {
            use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            p2d.remove("notes/a.md")?;
            p2d.insert("stale/a.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "stale/a.md")?;
        }
        write_txn.commit()?;
        Ok(())
    })
    .expect("poison metadata path");

    let sync = SyncManager::new(repo);
    sync.persist_doc(doc_id).expect("persist doc");

    let canonical = dir.path().join("vault").join("default").join("notes/a.md");
    assert_eq!(
        std::fs::read_to_string(canonical).expect("read canonical"),
        "hello"
    );
    assert!(
        !dir.path()
            .join("vault")
            .join("default")
            .join("stale/a.md")
            .exists()
    );
}

#[test]
fn persist_doc_fails_closed_when_tracked_projection_is_missing() {
    let (_dir, repo) = new_repo();
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), "notes/a.md", None, "test")
        .expect("create file structure");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        {
            let mut node_table = write_txn.open_table(NODEID_TO_META)?;
            node_table.remove(NodeId::from_doc_id(doc_id).as_u128())?;
        }
        write_txn.commit()?;
        Ok::<_, anyhow::Error>(())
    })
    .expect("remove tracked node meta");

    let sync = SyncManager::new(repo);
    let err = sync
        .persist_doc(doc_id)
        .expect_err("missing tracked projection must fail closed");
    assert!(
        err.to_string()
            .contains("Tracked document projection missing"),
        "unexpected error: {err:#}"
    );
}
