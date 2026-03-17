use super::{
    LegacyLedgerEntryV1, LegacyLedgerEntryV2, LegacyLedgerEntryV4, LegacyOpV1, LegacyOpV3,
    LegacyOpV4, deserialize_ledger_entry,
};
use crate::models::{ContentOp, DocId, PeerId};

#[test]
fn deserializes_latest_legacy_content_entry() -> anyhow::Result<()> {
    let legacy = LegacyLedgerEntryV4 {
        doc_id: DocId::from_u128(7),
        op: LegacyOpV4::Insert {
            pos: 0,
            content: "legacy".into(),
        },
        timestamp: 1,
        peer_id: PeerId::new("legacy"),
        seq: 9,
        client_id: Some(2),
        client_op_id: Some(3),
    };
    let bytes = bincode::serialize(&legacy)?;
    let entry = deserialize_ledger_entry(&bytes)?;
    assert_eq!(entry.doc_id, Some(legacy.doc_id));
    assert_eq!(entry.client_id, Some(2));
    assert_eq!(entry.client_op_id, Some(3));
    assert_eq!(
        entry.content_op(),
        Some(&ContentOp::Insert {
            pos: 0,
            content: "legacy".into(),
        })
    );
    Ok(())
}

#[test]
fn deserializes_peer_seq_legacy_entry() -> anyhow::Result<()> {
    let legacy = LegacyLedgerEntryV2 {
        doc_id: DocId::from_u128(8),
        op: LegacyOpV3::Delete { pos: 4, len: 2 },
        timestamp: 2,
        peer_id: PeerId::new("peer"),
        seq: 11,
    };
    let bytes = bincode::serialize(&legacy)?;
    let entry = deserialize_ledger_entry(&bytes)?;
    assert_eq!(entry.doc_id, Some(legacy.doc_id));
    assert_eq!(entry.peer_id, legacy.peer_id);
    assert_eq!(entry.seq, legacy.seq);
    assert_eq!(
        entry.content_op(),
        Some(&ContentOp::Delete { pos: 4, len: 2 })
    );
    Ok(())
}

#[test]
fn deserializes_oldest_legacy_entry() -> anyhow::Result<()> {
    let legacy = LegacyLedgerEntryV1 {
        doc_id: DocId::from_u128(9),
        op: LegacyOpV1::Insert {
            pos: 1,
            content: "oldest".to_string(),
        },
        timestamp: 3,
    };
    let bytes = bincode::serialize(&legacy)?;
    let entry = deserialize_ledger_entry(&bytes)?;
    assert_eq!(entry.doc_id, Some(legacy.doc_id));
    assert_eq!(entry.seq, 0);
    assert_eq!(entry.peer_id.as_str(), "legacy_local");
    assert_eq!(
        entry.content_op(),
        Some(&ContentOp::Insert {
            pos: 1,
            content: "oldest".into(),
        })
    );
    Ok(())
}
