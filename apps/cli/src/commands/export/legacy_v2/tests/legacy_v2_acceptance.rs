// Acceptance binding: STORE-014B.

use super::super::{
    LEGACY_ENTRY_MAGIC, LEGACY_SCHEMA_VERSION, LegacyEntryEnvelope, LegacyLedgerEntry,
    run as run_legacy_export,
};
use super::write_v2_fixture;
use anyhow::Result;
use deve_core::codec;
use deve_core::ledger::schema::LEDGER_OPS;
use deve_core::models::{DocId, LedgerEvent, NodeId, PeerId, StructureOp};
use std::path::Path;

fn append_v2_entry(db_path: &Path, global_seq: u64, entry: LegacyLedgerEntry) -> Result<()> {
    let db = redb::Database::open(db_path)?;
    let write = db.begin_write()?;
    {
        let mut ledger = write.open_table(LEDGER_OPS)?;
        let envelope = LegacyEntryEnvelope {
            format_version: LEGACY_SCHEMA_VERSION,
            entry,
        };
        let payload = codec::encode(&envelope)?;
        let mut bytes = LEGACY_ENTRY_MAGIC.to_vec();
        bytes.extend(payload);
        ledger.insert(global_seq, bytes.as_slice())?;
    }
    write.commit()?;
    Ok(())
}

#[test]
fn legacy_v2_export_without_explicit_flag_fails_closed() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    write_v2_fixture(&ledger_dir)?;
    let output = dir.path().join("legacy.jsonl");

    let error = super::super::super::run(
        &ledger_dir,
        Some(output.display().to_string()),
        Some("default".into()),
        None,
        100,
        "json",
        false,
        false,
    )
    .expect_err("normal export must reject a schema-v2 repo");
    let message = format!("{error:#}");
    assert!(
        message.contains("Uncataloged local authority artifacts require explicit ownership repair"),
        "normal UUID-first admission must reject the legacy filename before any export: {message}"
    );
    assert!(!output.exists());
    Ok(())
}

#[test]
fn legacy_v2_export_markdown_rejects_invalid_structure() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let (_existing_doc_id, db_path) = write_v2_fixture(&ledger_dir)?;
    let invalid_doc_id = DocId::new();
    let invalid_node_id = NodeId::from_doc_id(invalid_doc_id);
    append_v2_entry(
        &db_path,
        4,
        LegacyLedgerEntry {
            doc_id: Some(invalid_doc_id),
            event: LedgerEvent::Structure(StructureOp::CreateFile {
                node_id: invalid_node_id,
                doc_id: invalid_doc_id,
                parent_id: Some(invalid_node_id),
                name: "invalid.md".into(),
            }),
            timestamp: 4,
            peer_id: PeerId::new("legacy-label"),
            seq: 2,
            client_id: None,
            client_op_id: None,
        },
    )?;
    let output = dir.path().join("recovered");

    let error = run_legacy_export(
        &ledger_dir,
        Some(output.display().to_string()),
        Some("default".into()),
        None,
        "markdown",
    )
    .expect_err("invalid legacy structure must fail closed");
    assert!(
        format!("{error:#}").contains("legacy structure cycle detected"),
        "unexpected error: {error:#}"
    );
    assert!(!output.exists());
    Ok(())
}
