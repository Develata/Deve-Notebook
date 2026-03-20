use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID};
use deve_core::models::{LedgerEntry, NodeId, Op, PeerId, StructureOp};
use deve_core::sync::SyncManager;
use redb::ReadableTable;
use tempfile::TempDir;

fn new_repo() -> (TempDir, std::sync::Arc<RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, std::sync::Arc::new(repo))
}

fn seed_file(repo: &RepoManager, doc_path: &str, content: &str) {
    let doc_id = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), doc_path, None, "test")
        .expect("create file");
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
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
}

fn inject_legacy_doc_path(repo: &RepoManager, doc_id: deve_core::models::DocId, path: &str) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        {
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            d2p.insert(doc_id.as_u128(), path)?;
            p2d.insert(path, doc_id.as_u128())?;
        }
        write.commit()?;
        Ok(())
    })
    .expect("inject legacy doc path");
}

fn wipe_node_projection(repo: &RepoManager) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let read = db.begin_read()?;
        let node_paths = {
            let table = read.open_table(PATH_TO_NODEID)?;
            let mut keys = Vec::new();
            for item in table.iter()? {
                let (path, _) = item?;
                keys.push(path.value().to_string());
            }
            keys
        };
        let node_ids = {
            let table = read.open_table(NODEID_TO_META)?;
            let mut keys = Vec::new();
            for item in table.iter()? {
                let (node_id, _) = item?;
                keys.push(node_id.value());
            }
            keys
        };
        drop(read);

        let write = db.begin_write()?;
        {
            let mut n2m = write.open_table(NODEID_TO_META)?;
            let mut p2n = write.open_table(PATH_TO_NODEID)?;
            for path in node_paths {
                p2n.remove(path.as_str())?;
            }
            for node_id in node_ids {
                n2m.remove(node_id)?;
            }
        }
        write.commit()?;
        Ok(())
    })
    .expect("wipe node projection");
}

#[test]
fn rebuild_projection_rewrites_projection_tables_from_structure_facts() {
    let (dir, repo) = new_repo();
    repo.apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes/sub", "test")
        .expect("create dir");
    seed_file(repo.as_ref(), "notes/sub/a.md", "ledger");
    let doc_id = repo
        .get_docid("notes/sub/a.md")
        .expect("lookup doc")
        .expect("doc id");
    inject_legacy_doc_path(repo.as_ref(), doc_id, "stale/a.md");
    wipe_node_projection(repo.as_ref());

    let sync = SyncManager::new(repo.clone(), dir.path().join("vault"));
    sync.rebuild_projection_local_repo("default")
        .expect("rebuild projection tables");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let read = db.begin_read()?;
        let p2d = read.open_table(PATH_TO_DOCID)?;
        let d2p = read.open_table(DOCID_TO_PATH)?;
        let p2n = read.open_table(PATH_TO_NODEID)?;
        let n2m = read.open_table(NODEID_TO_META)?;

        assert!(p2d.get("stale/a.md")?.is_none());
        assert!(p2n.get("stale/a.md")?.is_none());
        assert_eq!(
            d2p.get(doc_id.as_u128())?.map(|v| v.value().to_string()),
            Some("notes/sub/a.md".to_string())
        );
        let node_id = p2n
            .get("notes/sub/a.md")?
            .map(|v| v.value())
            .expect("canonical node mapping");
        let meta: deve_core::models::NodeMeta =
            bincode::deserialize(n2m.get(node_id)?.expect("node meta").value())?;
        assert_eq!(meta.path, "notes/sub/a.md");
        assert_eq!(meta.doc_id, Some(doc_id));
        Ok(())
    })
    .expect("verify rebuilt projection");
}

#[test]
fn rebuild_projection_fails_closed_on_missing_structure_targets() {
    let (dir, repo) = new_repo();
    repo.append_local_op(&LedgerEntry::new_structure(
        StructureOp::RenameNode {
            node_id: NodeId::new(),
            doc_id: None,
            new_name: "broken".into(),
        },
        1,
        PeerId::new("test"),
        1,
    ))
    .expect("append malformed structure op");

    let sync = SyncManager::new(repo, dir.path().join("vault"));
    let err = sync
        .rebuild_projection_local_repo("default")
        .expect_err("malformed structure facts must fail closed");
    assert!(err.to_string().contains("missing node"));
}
