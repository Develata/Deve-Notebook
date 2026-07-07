use super::{
    LEDGER_ENTRY_FORMAT_MAGIC, LEDGER_ENTRY_FORMAT_VERSION, LedgerEntryEnvelope,
    deserialize_ledger_entry, serialize_ledger_entry,
};
use crate::codec;
use crate::models::{ContentOp, DocId, LedgerEntry, PeerId};

fn entry() -> LedgerEntry {
    LedgerEntry::new_content(
        DocId::from_u128(7),
        ContentOp::Insert {
            pos: 0,
            content: "v2".into(),
        },
        1,
        PeerId::new("peer-a"),
        9,
        Some(2),
        Some(3),
    )
}

#[test]
fn ledger_entry_format_roundtrips_v2_envelope() -> anyhow::Result<()> {
    let entry = entry();
    let bytes = serialize_ledger_entry(&entry)?;

    assert!(bytes.starts_with(LEDGER_ENTRY_FORMAT_MAGIC));
    let decoded = deserialize_ledger_entry(&bytes)?;
    assert_eq!(decoded.doc_id, entry.doc_id);
    assert_eq!(decoded.event, entry.event);
    assert_eq!(decoded.peer_id, entry.peer_id);
    assert_eq!(decoded.seq, entry.seq);
    assert_eq!(decoded.client_id, entry.client_id);
    assert_eq!(decoded.client_op_id, entry.client_op_id);
    Ok(())
}

#[test]
fn ledger_entry_format_rejects_unversioned_payload() {
    let bytes = b"not-a-versioned-ledger-entry".to_vec();

    let err = deserialize_ledger_entry(&bytes).expect_err("unversioned entry must fail closed");

    assert!(err.to_string().contains("missing DEVELDG2 magic"));
}

#[test]
fn ledger_entry_format_rejects_unsupported_version() -> anyhow::Result<()> {
    let envelope = LedgerEntryEnvelope {
        format_version: LEDGER_ENTRY_FORMAT_VERSION + 1,
        entry: entry(),
    };
    let payload = codec::encode(&envelope)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(LEDGER_ENTRY_FORMAT_MAGIC);
    bytes.extend(payload);

    let err =
        deserialize_ledger_entry(&bytes).expect_err("unsupported entry version must fail closed");

    assert!(
        err.to_string()
            .contains("unsupported ledger entry format version")
    );
    Ok(())
}
