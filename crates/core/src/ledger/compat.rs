use crate::models::{DocId, LedgerEntry, Op, PeerId};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyPeerId(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
enum LegacyOp {
    Insert { pos: usize, content: String },
    Delete { pos: usize, len: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyLedgerEntry {
    doc_id: DocId,
    op: LegacyOp,
    timestamp: i64,
    peer_id: LegacyPeerId,
    seq: u64,
}

impl TryFrom<LegacyLedgerEntry> for LedgerEntry {
    type Error = anyhow::Error;

    fn try_from(value: LegacyLedgerEntry) -> Result<Self> {
        let op = match value.op {
            LegacyOp::Insert { pos, content } => Op::Insert {
                pos: u32::try_from(pos).map_err(|_| anyhow!("Legacy op pos overflow: {}", pos))?,
                content: content.into(),
            },
            LegacyOp::Delete { pos, len } => Op::Delete {
                pos: u32::try_from(pos).map_err(|_| anyhow!("Legacy op pos overflow: {}", pos))?,
                len: u32::try_from(len).map_err(|_| anyhow!("Legacy op len overflow: {}", len))?,
            },
        };

        Ok(LedgerEntry {
            doc_id: value.doc_id,
            op,
            timestamp: value.timestamp,
            peer_id: PeerId::new(value.peer_id.0),
            seq: value.seq,
        })
    }
}

pub fn decode_entry(bytes: &[u8]) -> Result<LedgerEntry> {
    if let Ok(entry) = bincode::deserialize::<LedgerEntry>(bytes) {
        return Ok(entry);
    }

    let legacy =
        bincode::deserialize::<LegacyLedgerEntry>(bytes).context("legacy decode failed")?;
    legacy.try_into()
}
