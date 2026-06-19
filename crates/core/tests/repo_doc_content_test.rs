use deve_core::ledger::RepoManager;
use deve_core::ledger::node_meta::{ensure_file_node, remove_node_by_path};
use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS};
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId, serialize_ledger_entry};
use tempfile::TempDir;

#[test]
fn repo_doc_content_rejects_deleted_docs_even_if_ops_remain() {
    let dir = TempDir::new().expect("tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 8, None, None).expect("init repo");
    let doc_id = DocId::new();

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        ensure_file_node(db, "notes/a.md", doc_id)?;
        let entry = LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "hello".into(),
            },
            1,
            PeerId::new("peer-a"),
            1,
            None,
            None,
        );
        let bytes = serialize_ledger_entry(&entry)?;
        let write = db.begin_write()?;
        write
            .open_table(LEDGER_OPS)?
            .insert(1u64, bytes.as_slice())?;
        write
            .open_multimap_table(DOC_OPS)?
            .insert(doc_id.as_u128(), 1u64)?;
        write.commit()?;
        remove_node_by_path(db, "notes/a.md")?;
        Ok(())
    })
    .expect("seed deleted doc");

    let err = Repository::get_doc_content_in_repo(
        &repo,
        &RepoSelector {
            repo_id: None,
            repo_name: None,
        },
        doc_id,
    )
    .expect_err("deleted doc must not resolve content");

    assert!(err.to_string().contains("Document not found"));
}
