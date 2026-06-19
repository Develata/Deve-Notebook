//! plan_ref:
//!   - 03_storage/authority#ledger-entry-format-contract

use super::ledger_event::LedgerEntry;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub const LEDGER_ENTRY_FORMAT_MAGIC: &[u8; 8] = b"DEVELDG1";
pub const LEDGER_ENTRY_FORMAT_VERSION: u16 = 1;

#[derive(Serialize, Deserialize)]
struct LedgerEntryEnvelope {
    format_version: u16,
    entry: LedgerEntry,
}

pub fn serialize_ledger_entry(entry: &LedgerEntry) -> Result<Vec<u8>> {
    let envelope = LedgerEntryEnvelope {
        format_version: LEDGER_ENTRY_FORMAT_VERSION,
        entry: entry.clone(),
    };
    let payload = bincode::serialize(&envelope).context("failed to serialize ledger entry v1")?;
    let mut bytes = Vec::with_capacity(LEDGER_ENTRY_FORMAT_MAGIC.len() + payload.len());
    bytes.extend_from_slice(LEDGER_ENTRY_FORMAT_MAGIC);
    bytes.extend(payload);
    Ok(bytes)
}

pub fn deserialize_ledger_entry(bytes: &[u8]) -> Result<LedgerEntry> {
    let payload = bytes
        .strip_prefix(LEDGER_ENTRY_FORMAT_MAGIC)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "unsupported ledger entry format: missing DEVELDG1 magic ({} bytes)",
                bytes.len()
            )
        })?;
    let envelope: LedgerEntryEnvelope =
        bincode::deserialize(payload).context("failed to deserialize ledger entry envelope")?;
    if envelope.format_version != LEDGER_ENTRY_FORMAT_VERSION {
        bail!(
            "unsupported ledger entry format version {}; expected {}",
            envelope.format_version,
            LEDGER_ENTRY_FORMAT_VERSION
        );
    }
    Ok(envelope.entry)
}

#[cfg(test)]
mod tests;
