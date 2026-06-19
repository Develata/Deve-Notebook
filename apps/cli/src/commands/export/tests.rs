use super::run;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS, NODE_OPS, PEER_DOC_SEQ};
use deve_core::models::{
    DocId, LedgerEntry, NodeId, Op, PeerId, StructureOp, serialize_ledger_entry,
};
use redb::ReadableTable;
use tempfile::TempDir;

fn seed_doc(repo: &RepoManager, path: &str, content: &str) -> DocId {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), path, None, "test")
        .expect("structure");
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
            let mut peer_seqs = write.open_table(PEER_DOC_SEQ)?;
            let next_seq = ops.last()?.map(|(key, _)| key.value() + 1).unwrap_or(1);
            let bytes = serialize_ledger_entry(entry)?;
            ops.insert(next_seq, bytes.as_slice())?;
            if let Some(doc_id) = entry.doc_id {
                doc_ops.insert(doc_id.as_u128(), next_seq)?;
                peer_seqs.insert((doc_id.as_u128(), entry.peer_id.as_str()), entry.seq)?;
            }
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
fn markdown_export_supports_single_doc_output() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, None, None).expect("init repo");
    let doc_id = seed_doc(&repo, "notes/a.md", "hello export");
    let output = dir.path().join("single.md");

    run(
        &ledger_dir,
        Some(output.display().to_string()),
        None,
        Some(doc_id.to_string()),
        8,
        "markdown",
        false,
    )
    .expect("export markdown doc");

    assert_eq!(
        std::fs::read_to_string(output).expect("read export"),
        "hello export"
    );
}

#[test]
fn markdown_export_preserves_user_frontmatter_without_system_metadata() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, None, None).expect("init repo");
    let content = "---\ntitle: User Note\n---\nbody";
    let doc_id = seed_doc(&repo, "notes/frontmatter.md", content);
    let output = dir.path().join("frontmatter.md");

    run(
        &ledger_dir,
        Some(output.display().to_string()),
        None,
        Some(doc_id.to_string()),
        8,
        "markdown",
        false,
    )
    .expect("export markdown doc");

    let exported = std::fs::read_to_string(output).expect("read export");
    assert_eq!(exported, content);
    assert!(!exported.contains("doc_id"));
    assert!(!exported.contains("node_id"));
    assert!(!exported.contains("repo_id"));
    assert!(!exported.contains("uuid:"));
}

#[test]
fn json_export_rejects_single_doc_selector() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let err = run(
        &ledger_dir,
        None,
        None,
        Some(uuid::Uuid::new_v4().to_string()),
        8,
        "json",
        false,
    )
    .expect_err("json export should reject --doc");

    assert!(
        err.to_string()
            .contains("JSON export does not support --doc"),
        "unexpected error: {err:#}"
    );
}

#[test]
fn markdown_export_requires_explicit_degraded_projection_flag() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, None, None).expect("init repo");
    seed_doc(&repo, "notes/a.md", "safe content");
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

    let output = dir.path().join("export");
    let err = run(
        &ledger_dir,
        Some(output.display().to_string()),
        None,
        None,
        8,
        "markdown",
        false,
    )
    .expect_err("degraded projection export must require explicit flag");

    assert!(
        err.to_string().contains("--allow-degraded-projection"),
        "unexpected error: {err:#}"
    );
    run(
        &ledger_dir,
        Some(output.display().to_string()),
        None,
        None,
        8,
        "markdown",
        true,
    )
    .expect("explicit degraded export");
    assert_eq!(
        std::fs::read_to_string(output.join("notes/a.md")).expect("read exported doc"),
        "safe content"
    );
}
