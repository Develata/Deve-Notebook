//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 04_repository#tree-projection-contract

use super::check_repair_readiness;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS, NODE_OPS, PEER_FACT_OPS, PEER_FACT_SEQ};
use deve_core::models::{DocId, LedgerEntry, NodeId, PeerId, StructureOp, serialize_ledger_entry};
use redb::ReadableTable;
use std::sync::Arc;
use tempfile::TempDir;

fn new_repo() -> anyhow::Result<(TempDir, Arc<RepoManager>)> {
    let dir = TempDir::new()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    Ok((dir, Arc::new(cataloged.repo)))
}

fn append_unvalidated(repo: &RepoManager, entry: &LedgerEntry) -> anyhow::Result<()> {
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
}

#[test]
fn repair_check_fails_closed_on_authority_corrupt_projection() -> anyhow::Result<()> {
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

    let repo_name = repo.local_repo_name().to_owned();
    let err = check_repair_readiness(repo, std::slice::from_ref(&repo_name))
        .expect_err("authority corruption must keep repair preflight fail-closed");

    assert!(
        err.to_string()
            .contains("repair-check: 1 repo(s) have corrupted Structure Facts authority")
    );
    assert!(
        err.to_string()
            .contains("repair steps must remain disabled")
    );
    assert!(
        err.to_string()
            .contains(&format!("{repo_name}:missing_parent"))
    );
    Ok(())
}
