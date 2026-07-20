// crates\core\src\ledger
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 07_network#server-ws-runtime
//!
//! # 范围查询模块 (Range Query Operations)
//!
//! **架构作用**:
//! 提供基于序列号范围的操作查询功能，用于 P2P 同步的增量数据拉取和推送。
//!
//! **核心功能清单**:
//! - `get_ops_in_range`: 从指定数据库获取范围内的操作。
//! - `get_max_seq`: 获取数据库的最大序列号。
//!
//! **类型**: Core MUST (核心必选)

use crate::ledger::schema::{LEDGER_OPS, PEER_FACT_OPS, PEER_FACT_SEQ};
use crate::models::{LedgerEntry, PeerFactSeq, PeerId, deserialize_ledger_entry};
use crate::protocol::{MAX_SYNC_FACT_BYTES_PER_PAYLOAD, MAX_SYNC_FACTS_PER_PAYLOAD};
use anyhow::{Context, Result, anyhow};
use redb::{Database, ReadableTable};

/// 从数据库获取指定序列号范围的操作
pub fn get_ops_in_range(
    db: &Database,
    start_seq: u64,
    end_seq: u64,
) -> Result<Vec<(u64, LedgerEntry)>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(LEDGER_OPS)?;

    let mut result = Vec::new();
    let range = table.range(start_seq..end_seq)?;
    for item in range {
        let (key, value) = item?;
        let seq = key.value();
        let entry: LedgerEntry = deserialize_ledger_entry(value.value())
            .with_context(|| format!("Failed to deserialize op at seq {}", seq))?;
        result.push((seq, entry));
    }
    Ok(result)
}

/// 获取数据库的最大序列号
pub fn get_max_seq(db: &Database) -> Result<u64> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(LEDGER_OPS)?;

    let last = table.last()?;
    Ok(last.map(|(k, _)| k.value()).unwrap_or(0))
}

pub fn get_peer_waterline(db: &Database, peer_id: &PeerId) -> Result<PeerFactSeq> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PEER_FACT_SEQ)?;
    Ok(table
        .get(peer_id.as_str())?
        .map(|value| PeerFactSeq::new(value.value()))
        .unwrap_or(PeerFactSeq::ZERO))
}

/// Resolve an exact closed PeerFactSeq range. Missing index rows fail closed.
pub fn get_peer_ops_in_range(
    db: &Database,
    peer_id: &PeerId,
    start: PeerFactSeq,
    end: PeerFactSeq,
) -> Result<Vec<(u64, LedgerEntry)>> {
    if start == PeerFactSeq::ZERO || end < start {
        return Err(anyhow!(
            "invalid closed peer fact range for {}: {}..={}",
            peer_id,
            start,
            end
        ));
    }
    let read_txn = db.begin_read()?;
    let peer_ops = read_txn.open_table(PEER_FACT_OPS)?;
    let ledger_ops = read_txn.open_table(LEDGER_OPS)?;
    let waterline = read_txn
        .open_table(PEER_FACT_SEQ)?
        .get(peer_id.as_str())?
        .map(|value| PeerFactSeq::new(value.value()))
        .unwrap_or(PeerFactSeq::ZERO);
    if end > waterline {
        return Err(anyhow!(
            "sequence_gap: source={} requested_end={} waterline={}",
            peer_id,
            end,
            waterline
        ));
    }
    let width = end
        .get()
        .checked_sub(start.get())
        .and_then(|delta| delta.checked_add(1))
        .ok_or_else(|| anyhow!("peer fact range width overflow: {}..={}", start, end))?;
    if width > MAX_SYNC_FACTS_PER_PAYLOAD {
        return Err(anyhow!(
            "sync_resource_limit: range {}..={} contains {} facts; limit={}",
            start,
            end,
            width,
            MAX_SYNC_FACTS_PER_PAYLOAD
        ));
    }
    let capacity = usize::try_from(width)
        .map_err(|_| anyhow!("sync_resource_limit: range width does not fit usize"))?;
    let mut result = Vec::with_capacity(capacity);
    let mut encoded_bytes = 0_u64;
    let mut seq = start;
    loop {
        let global_seq = peer_ops
            .get((peer_id.as_str(), seq.get()))?
            .ok_or_else(|| {
                anyhow!(
                    "sequence_gap: source={} expected={} range={}..={}",
                    peer_id,
                    seq,
                    start,
                    end
                )
            })?
            .value();
        let bytes = ledger_ops.get(global_seq)?.ok_or_else(|| {
            anyhow!(
                "sequence_gap: source={} peer_seq={} references missing GlobalSeq {}",
                peer_id,
                seq,
                global_seq
            )
        })?;
        encoded_bytes = encoded_bytes
            .checked_add(bytes.value().len() as u64)
            .ok_or_else(|| anyhow!("sync_resource_limit: encoded fact byte count overflow"))?;
        if encoded_bytes > MAX_SYNC_FACT_BYTES_PER_PAYLOAD {
            return Err(anyhow!(
                "sync_resource_limit: encoded facts exceed {} bytes",
                MAX_SYNC_FACT_BYTES_PER_PAYLOAD
            ));
        }
        let entry = deserialize_ledger_entry(bytes.value())
            .with_context(|| format!("Failed to deserialize op at GlobalSeq {global_seq}"))?;
        if entry.origin_peer_id != *peer_id || entry.peer_seq != seq {
            return Err(anyhow!(
                "peer fact index mismatch: source={} expected_seq={} payload_source={} payload_seq={}",
                peer_id,
                seq,
                entry.origin_peer_id,
                entry.peer_seq
            ));
        }
        result.push((global_seq, entry));
        if seq == end {
            break;
        }
        seq = seq
            .next()
            .ok_or_else(|| anyhow!("PeerFactSeq overflow while reading range"))?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untrusted_huge_range_fails_before_capacity_allocation() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let (repo, _repo_id) = crate::test_support::init_cataloged_repo_with_depth(
            &dir.path().join("ledger"),
            &dir.path().join("notes"),
            10,
        )?;
        let error = repo.run_on_local_repo(repo.local_repo_name(), |db| {
            get_peer_ops_in_range(
                db,
                repo.local_peer_id(),
                PeerFactSeq::ONE,
                PeerFactSeq::new(u64::MAX),
            )
        });
        assert!(
            error
                .expect_err("range beyond waterline must fail")
                .to_string()
                .contains("sequence_gap")
        );
        Ok(())
    }
}
