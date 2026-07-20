use super::{collect_projection_reports, print_projection_reports};
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
fn projection_node_check_reports_authority_corruption() -> anyhow::Result<()> {
    let (_dir, repo) = new_repo()?;
    let execution_name = repo.local_repo_name().to_owned();
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

    let reports = collect_projection_reports(repo, Some(&execution_name))?;

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, "authority_corrupt");
    assert_eq!(reports[0].issue_code.as_deref(), Some("missing_parent"));
    assert!(!reports[0].rebuild_supported);
    assert!(reports[0].repair_hint.contains("rebuild is unsupported"));
    let err =
        print_projection_reports(&reports).expect_err("authority corruption must fail closed");
    assert!(
        err.to_string()
            .contains("projection check failed closed: Structure Facts authority corrupt")
    );
    assert!(
        err.to_string()
            .contains(&format!("{execution_name}:missing_parent"))
    );
    Ok(())
}

#[test]
fn projection_node_check_missing_locator_fails_closed_without_panic() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let cataloged = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )?;
    let repo_id = cataloged.repo_id;
    let repo = Arc::new(cataloged.repo);
    // Catalog the repo, then remove exactly the projection locator so the repo
    // still resolves but projection diagnosis fails closed on the missing locator.
    repo.remove_projection_locator_for_repo_id(repo_id)?;

    let err = collect_projection_reports(repo, Some(&repo_id.to_string()))
        .expect_err("missing locator must be returned as an error");

    assert!(
        err.to_string().contains("Projection Locator missing"),
        "unexpected error: {err}"
    );
    Ok(())
}
