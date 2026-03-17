use super::ledger_event::{ContentOp, LedgerEntry, LedgerEvent};
use super::{DocId, PeerId};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Serialize, Deserialize)]
enum LegacyOpV4 {
    Insert { pos: u32, content: SmolStr },
    Delete { pos: u32, len: u32 },
}

impl LegacyOpV4 {
    fn into_current(self) -> ContentOp {
        match self {
            Self::Insert { pos, content } => ContentOp::Insert { pos, content },
            Self::Delete { pos, len } => ContentOp::Delete { pos, len },
        }
    }
}

#[derive(Serialize, Deserialize)]
enum LegacyOpV3 {
    Insert { pos: usize, content: SmolStr },
    Delete { pos: usize, len: usize },
}

impl LegacyOpV3 {
    fn into_current(self) -> Result<ContentOp> {
        let conv = |n: usize| u32::try_from(n).context("legacy usize index exceeds u32");
        Ok(match self {
            Self::Insert { pos, content } => ContentOp::Insert {
                pos: conv(pos)?,
                content,
            },
            Self::Delete { pos, len } => ContentOp::Delete {
                pos: conv(pos)?,
                len: conv(len)?,
            },
        })
    }
}

#[derive(Serialize, Deserialize)]
enum LegacyOpV1 {
    Insert { pos: usize, content: String },
    Delete { pos: usize, len: usize },
}

impl LegacyOpV1 {
    fn into_current(self) -> Result<ContentOp> {
        let conv = |n: usize| u32::try_from(n).context("legacy usize index exceeds u32");
        Ok(match self {
            Self::Insert { pos, content } => ContentOp::Insert {
                pos: conv(pos)?,
                content: content.into(),
            },
            Self::Delete { pos, len } => ContentOp::Delete {
                pos: conv(pos)?,
                len: conv(len)?,
            },
        })
    }
}

#[derive(Serialize, Deserialize)]
struct LegacyLedgerEntryV4 {
    doc_id: DocId,
    op: LegacyOpV4,
    timestamp: i64,
    peer_id: PeerId,
    seq: u64,
    client_id: Option<u64>,
    client_op_id: Option<u64>,
}

impl LegacyLedgerEntryV4 {
    fn into_current(self) -> LedgerEntry {
        legacy_entry(
            self.doc_id,
            self.op.into_current(),
            self.timestamp,
            self.peer_id,
            self.seq,
            self.client_id,
            self.client_op_id,
        )
    }
}

#[derive(Serialize, Deserialize)]
struct LegacyLedgerEntryV3 {
    doc_id: DocId,
    op: LegacyOpV4,
    timestamp: i64,
    peer_id: PeerId,
    seq: u64,
}

impl LegacyLedgerEntryV3 {
    fn into_current(self) -> LedgerEntry {
        legacy_entry(
            self.doc_id,
            self.op.into_current(),
            self.timestamp,
            self.peer_id,
            self.seq,
            None,
            None,
        )
    }
}

#[derive(Serialize, Deserialize)]
struct LegacyLedgerEntryV2 {
    doc_id: DocId,
    op: LegacyOpV3,
    timestamp: i64,
    peer_id: PeerId,
    seq: u64,
}

impl LegacyLedgerEntryV2 {
    fn into_current(self) -> Result<LedgerEntry> {
        Ok(legacy_entry(
            self.doc_id,
            self.op.into_current()?,
            self.timestamp,
            self.peer_id,
            self.seq,
            None,
            None,
        ))
    }
}

#[derive(Serialize, Deserialize)]
struct LegacyLedgerEntryV1 {
    doc_id: DocId,
    op: LegacyOpV1,
    timestamp: i64,
}

impl LegacyLedgerEntryV1 {
    fn into_current(self) -> Result<LedgerEntry> {
        Ok(legacy_entry(
            self.doc_id,
            self.op.into_current()?,
            self.timestamp,
            PeerId::new("legacy_local"),
            0,
            None,
            None,
        ))
    }
}

fn legacy_entry(
    doc_id: DocId,
    op: ContentOp,
    timestamp: i64,
    peer_id: PeerId,
    seq: u64,
    client_id: Option<u64>,
    client_op_id: Option<u64>,
) -> LedgerEntry {
    LedgerEntry {
        doc_id: Some(doc_id),
        event: LedgerEvent::content(op),
        timestamp,
        peer_id,
        seq,
        client_id,
        client_op_id,
    }
}

pub fn deserialize_ledger_entry(bytes: &[u8]) -> Result<LedgerEntry> {
    if let Some(entry) = exact_decode::<LedgerEntry>(bytes) {
        return Ok(entry);
    }
    if let Some(entry) = exact_decode::<LegacyLedgerEntryV4>(bytes) {
        return Ok(entry.into_current());
    }
    if let Some(entry) = exact_decode::<LegacyLedgerEntryV3>(bytes) {
        return Ok(entry.into_current());
    }
    if let Some(entry) = exact_decode::<LegacyLedgerEntryV2>(bytes) {
        return entry.into_current();
    }
    if let Some(entry) = exact_decode::<LegacyLedgerEntryV1>(bytes) {
        return entry.into_current();
    }
    Err(anyhow::anyhow!(
        "unsupported ledger entry schema ({} bytes)",
        bytes.len()
    ))
}

fn exact_decode<T>(bytes: &[u8]) -> Option<T>
where
    T: DeserializeOwned + Serialize,
{
    let value = bincode::deserialize::<T>(bytes).ok()?;
    let roundtrip = bincode::serialize(&value).ok()?;
    (roundtrip == bytes).then_some(value)
}

#[cfg(test)]
#[path = "ledger_decode_test.rs"]
mod tests;
