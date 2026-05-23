use super::rebuild_repos;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS, NODE_OPS, PEER_DOC_SEQ};
use deve_core::models::{DocId, LedgerEntry, NodeId, PeerId, StructureOp};
use deve_core::sync::SyncManager;
use redb::ReadableTable;
use std::sync::Arc;
use tempfile::TempDir;

fn new_repo() -> anyhow::Result<(TempDir, Arc<RepoManager>)> {
    let dir = TempDir::new()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos(dir.path().join("vault"));
    Ok((dir, Arc::new(repo)))
}

fn append_unvalidated(repo: &RepoManager, entry: &LedgerEntry) -> anyhow::Result<()> {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        {
            let mut ops = write.open_table(LEDGER_OPS)?;
            let mut doc_ops = write.open_multimap_table(DOC_OPS)?;
            let mut node_ops = write.open_multimap_table(NODE_OPS)?;
            let mut peer_seqs = write.open_table(PEER_DOC_SEQ)?;
            let next_seq = ops.last()?.map(|(key, _)| key.value() + 1).unwrap_or(1);
            let bytes = bincode::serialize(entry)?;
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
}

#[test]
fn rebuild_repos_reports_authority_corrupt_without_rebuild() -> anyhow::Result<()> {
    let (_dir, repo) = new_repo()?;
    let doc_id = DocId::new();
    append_unvalidated(
        repo.as_ref(),
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::from_doc_id(doc_id),
                doc_id,
                parent_id: Some(NodeId::new()),
                name: "orphan.md".into(),
            },
            1,
            PeerId::new("test"),
            1,
        ),
    )?;

    let sync = SyncManager::new(repo);
    let report = rebuild_repos(&sync, &[String::from("default")])?;

    assert_eq!(report.rebuilt, 0);
    assert_eq!(report.authority_corrupt.len(), 1);
    assert_eq!(report.authority_corrupt[0].repo_name, "default");
    assert_eq!(report.authority_corrupt[0].code, "missing_parent");
    assert!(
        report.authority_corrupt[0]
            .detail
            .contains("missing parent")
    );
    Ok(())
}
