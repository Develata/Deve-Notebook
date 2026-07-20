use super::run;
use deve_core::graph::GraphProjection;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS, NODE_OPS, PEER_FACT_OPS, PEER_FACT_SEQ};
use deve_core::models::{
    DocId, FactActor, LedgerEntry, NodeId, Op, PeerId, StructureOp, serialize_ledger_entry,
};
use redb::ReadableTable;
use tempfile::TempDir;

fn seed_doc(repo: &RepoManager, path: &str, content: &str) -> DocId {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), path, None, "test")
        .expect("structure");
    repo.local_fact_writer(FactActor::new("test").expect("actor"))
        .append_content_in_local_repo(
            repo.local_repo_name(),
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
        )
        .expect("append op");
    doc_id
}

fn append_unvalidated(repo: &RepoManager, entry: &LedgerEntry) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        {
            let mut ops = write.open_table(LEDGER_OPS)?;
            let mut doc_ops = write.open_multimap_table(DOC_OPS)?;
            let mut node_ops = write.open_multimap_table(NODE_OPS)?;
            let mut peer_seqs = write.open_table(PEER_FACT_SEQ)?;
            let mut peer_ops = write.open_table(PEER_FACT_OPS)?;
            let next_seq = ops.last()?.map(|(key, _)| key.value() + 1).unwrap_or(1);
            let bytes = serialize_ledger_entry(entry)?;
            ops.insert(next_seq, bytes.as_slice())?;
            if let Some(doc_id) = entry.doc_id {
                doc_ops.insert(doc_id.as_u128(), next_seq)?;
            }
            peer_seqs.insert(entry.origin_peer_id.as_str(), entry.peer_seq.get())?;
            peer_ops.insert(
                (entry.origin_peer_id.as_str(), entry.peer_seq.get()),
                next_seq,
            )?;
            if let Some(node_id) = entry.structure_node_id() {
                node_ops.insert(node_id.as_u128(), next_seq)?;
            }
        }
        write.commit()?;
        Ok(())
    })
    .expect("append unvalidated")
}

#[test]
fn graph_command_writes_read_only_projection_json() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("notes"), 8)
        .expect("init repo")
        .repo;
    let b = seed_doc(&repo, "notes/b.md", "");
    seed_doc(&repo, "notes/a.md", "[[b]] and [B](b.md)");
    let output = dir.path().join("graph.json");

    run(
        &ledger_dir,
        None,
        Some(output.display().to_string()),
        true,
        false,
        8,
    )
    .expect("graph export");

    let json = std::fs::read_to_string(output).expect("read graph");
    let projection: GraphProjection = serde_json::from_str(&json).expect("parse graph");
    assert_eq!(projection.nodes.len(), 2);
    assert_eq!(projection.edges.len(), 2);
    assert!(projection.edges.iter().all(|edge| edge.to_doc_id == b));
    assert!(projection.unresolved_links.is_empty());
}

#[test]
fn graph_command_fails_closed_on_corrupt_structure_projection() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("notes"), 8)
        .expect("init repo")
        .repo;
    seed_doc(&repo, "notes/a.md", "safe");
    let orphan_doc = DocId::new();
    append_unvalidated(
        &repo,
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::from_doc_id(orphan_doc),
                doc_id: orphan_doc,
                parent_id: Some(NodeId::new()),
                name: "orphan.md".into(),
            },
            1,
            PeerId::new("test"),
            1,
        ),
    );

    let output = dir.path().join("graph.json");
    let err = run(
        &ledger_dir,
        None,
        Some(output.display().to_string()),
        false,
        false,
        8,
    )
    .expect_err("graph export must reject corrupt authority by default");

    assert!(
        err.to_string().contains("--allow-degraded-projection"),
        "unexpected error: {err:#}"
    );
    run(
        &ledger_dir,
        None,
        Some(output.display().to_string()),
        false,
        true,
        8,
    )
    .expect("explicit degraded graph export");
    let json = std::fs::read_to_string(output).expect("read graph");
    let projection: GraphProjection = serde_json::from_str(&json).expect("parse graph");
    assert_eq!(projection.nodes.len(), 1);
}
