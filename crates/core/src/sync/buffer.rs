// crates\core\src\sync
//! # 同步缓冲模块 (Sync Buffer)
//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! **架构作用**:
//! 管理 Manual 模式下的待处理操作队列。
//! 暂存从远端接收但尚未合并的操作。
//!
//! **核心功能清单**:
//! - `PendingOpsBuffer`: 待合并操作缓冲区。
//! - `push` / `push_snapshot`: 经累计资源门禁暂存 payload。
//! - `take`: 取出完整缓冲区，供合并失败时原样恢复。
//!
//! **类型**: Core MUST (核心必选)

use crate::protocol::{MAX_SYNC_FACT_BYTES_PER_PAYLOAD, MAX_SYNC_FACTS_PER_PAYLOAD};
use crate::sync::protocol::SyncResponse;
use anyhow::{Result, anyhow, bail};

const MAX_PENDING_PAYLOADS: u64 = MAX_SYNC_FACTS_PER_PAYLOAD;

/// Manual mode buffered payload kind.
#[derive(Clone)]
pub(super) enum PendingSyncPayload {
    Ops(SyncResponse),
    Snapshot(SyncResponse),
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PendingBufferAdmission {
    payload_count: u64,
    fact_count: u64,
    encoded_bytes: u64,
}

#[derive(Clone, Copy)]
struct PendingResourceLimits {
    payload_count: u64,
    fact_count: u64,
    encoded_bytes: u64,
}

const PENDING_RESOURCE_LIMITS: PendingResourceLimits = PendingResourceLimits {
    payload_count: MAX_PENDING_PAYLOADS,
    fact_count: MAX_SYNC_FACTS_PER_PAYLOAD,
    encoded_bytes: MAX_SYNC_FACT_BYTES_PER_PAYLOAD,
};

/// 待合并操作缓冲区
#[derive(Default, Clone)]
pub struct PendingOpsBuffer {
    /// 暂存的操作响应队列
    queue: Vec<PendingSyncPayload>,
    payload_count: u64,
    fact_count: u64,
    encoded_bytes: u64,
}

impl PendingOpsBuffer {
    /// 创建新的缓冲区
    pub fn new() -> Self {
        Self::default()
    }

    /// 暂存来自远端的同步响应
    pub fn push(&mut self, response: SyncResponse) -> Result<()> {
        let admission = self.preflight(&response)?;
        self.push_admitted(PendingSyncPayload::Ops(response), admission);
        Ok(())
    }

    /// 暂存来自远端的快照响应
    pub fn push_snapshot(&mut self, response: SyncResponse) -> Result<()> {
        let admission = self.preflight(&response)?;
        self.push_admitted(PendingSyncPayload::Snapshot(response), admission);
        Ok(())
    }

    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// 获取待合并的操作总数 (Entry 粒度)
    pub fn count(&self) -> usize {
        self.fact_count as usize
    }

    #[cfg(test)]
    pub(crate) fn payload_count(&self) -> u64 {
        self.payload_count
    }

    #[cfg(test)]
    pub(crate) fn encoded_bytes(&self) -> u64 {
        self.encoded_bytes
    }

    pub(super) fn payloads(&self) -> &[PendingSyncPayload] {
        &self.queue
    }

    /// 在解密或复制 payload 前计算整个 Manual queue 的新资源水位。
    pub(super) fn preflight(&self, response: &SyncResponse) -> Result<PendingBufferAdmission> {
        self.preflight_with_limits(response, PENDING_RESOURCE_LIMITS)
    }

    fn preflight_with_limits(
        &self,
        response: &SyncResponse,
        limits: PendingResourceLimits,
    ) -> Result<PendingBufferAdmission> {
        let fact_delta = u64::try_from(response.ops.len())
            .map_err(|_| anyhow!("sync_resource_limit: fact count does not fit u64"))?;
        let payload_count = checked_total(self.payload_count, 1, "payload count")?;
        let fact_count = checked_total(self.fact_count, fact_delta, "fact count")?;

        if payload_count > limits.payload_count {
            bail!(
                "sync_resource_limit: manual pending payload count {} exceeds {}",
                payload_count,
                limits.payload_count
            );
        }
        if fact_count > limits.fact_count {
            bail!(
                "sync_resource_limit: manual pending fact count {} exceeds {}",
                fact_count,
                limits.fact_count
            );
        }

        let mut encoded_bytes = self.encoded_bytes;
        for op in &response.ops {
            let op_bytes = u64::try_from(crate::codec::encoded_size(op)?)
                .map_err(|_| anyhow!("sync_resource_limit: encoded op size does not fit u64"))?;
            encoded_bytes = checked_total(encoded_bytes, op_bytes, "encoded bytes")?;
            if encoded_bytes > limits.encoded_bytes {
                bail!(
                    "sync_resource_limit: manual pending encoded bytes {} exceeds {}",
                    encoded_bytes,
                    limits.encoded_bytes
                );
            }
        }

        Ok(PendingBufferAdmission {
            payload_count,
            fact_count,
            encoded_bytes,
        })
    }

    pub(super) fn push_admitted(
        &mut self,
        payload: PendingSyncPayload,
        admission: PendingBufferAdmission,
    ) {
        self.queue.push(payload);
        self.payload_count = admission.payload_count;
        self.fact_count = admission.fact_count;
        self.encoded_bytes = admission.encoded_bytes;
    }

    /// 取出完整缓冲区；调用方可在失败时连同计数原样恢复。
    pub(super) fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

fn checked_total(current: u64, delta: u64, label: &str) -> Result<u64> {
    current
        .checked_add(delta)
        .ok_or_else(|| anyhow!("sync_resource_limit: manual pending {label} overflow"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PeerFactSeq, PeerId, RepoId};
    use crate::security::EncryptedOp;

    fn response(ops: Vec<EncryptedOp>) -> SyncResponse {
        SyncResponse::incremental(
            PeerId::new("remote"),
            RepoId::from_u128(1),
            (PeerFactSeq::ONE, PeerFactSeq::ONE),
            ops,
        )
    }

    fn encrypted_op(ciphertext_len: usize) -> EncryptedOp {
        EncryptedOp {
            doc_id: None,
            peer_seq: PeerFactSeq::ONE,
            ciphertext: vec![0; ciphertext_len],
            nonce: vec![0; 12],
        }
    }

    #[test]
    fn manual_pending_payload_fact_and_encoded_bytes_are_cumulative_and_bounded() -> Result<()> {
        let limits = PendingResourceLimits {
            payload_count: 2,
            fact_count: 2,
            encoded_bytes: 256,
        };
        let mut buffer = PendingOpsBuffer::new();
        let first = response(vec![encrypted_op(8)]);
        let first_bytes = crate::codec::encoded_size(&first.ops[0])? as u64;
        let admission = buffer.preflight_with_limits(&first, limits)?;
        buffer.push_admitted(PendingSyncPayload::Ops(first), admission);

        let second = response(vec![encrypted_op(8)]);
        let admission = buffer.preflight_with_limits(&second, limits)?;
        buffer.push_admitted(PendingSyncPayload::Snapshot(second), admission);
        assert_eq!(buffer.payload_count(), 2);
        assert_eq!(buffer.count(), 2);
        assert_eq!(buffer.encoded_bytes(), first_bytes * 2);

        let before = (
            buffer.payload_count(),
            buffer.count(),
            buffer.encoded_bytes(),
        );
        let error = buffer
            .preflight_with_limits(&response(Vec::new()), limits)
            .expect_err("one payload over the cumulative limit must fail");
        assert!(error.to_string().contains("sync_resource_limit"));
        assert_eq!(
            (
                buffer.payload_count(),
                buffer.count(),
                buffer.encoded_bytes()
            ),
            before
        );
        Ok(())
    }

    #[test]
    fn manual_pending_fact_and_byte_limits_reject_one_over_without_mutation() -> Result<()> {
        let op = encrypted_op(8);
        let op_bytes = crate::codec::encoded_size(&op)? as u64;
        assert_eq!(op_bytes, crate::codec::encode(&op)?.len() as u64);
        let mut buffer = PendingOpsBuffer::new();
        let fact_limits = PendingResourceLimits {
            payload_count: 3,
            fact_count: 1,
            encoded_bytes: u64::MAX,
        };
        let first = response(vec![op.clone()]);
        let admission = buffer.preflight_with_limits(&first, fact_limits)?;
        buffer.push_admitted(PendingSyncPayload::Ops(first), admission);

        let before = (
            buffer.payload_count(),
            buffer.count(),
            buffer.encoded_bytes(),
        );
        let error = buffer
            .preflight_with_limits(&response(vec![op.clone()]), fact_limits)
            .expect_err("one fact over the cumulative limit must fail");
        assert!(error.to_string().contains("sync_resource_limit"));
        assert_eq!(
            (
                buffer.payload_count(),
                buffer.count(),
                buffer.encoded_bytes()
            ),
            before
        );

        let byte_limits = PendingResourceLimits {
            payload_count: 3,
            fact_count: 3,
            encoded_bytes: op_bytes,
        };
        let mut buffer = PendingOpsBuffer::new();
        let first = response(vec![op]);
        let admission = buffer.preflight_with_limits(&first, byte_limits)?;
        buffer.push_admitted(PendingSyncPayload::Ops(first), admission);
        let before = (
            buffer.payload_count(),
            buffer.count(),
            buffer.encoded_bytes(),
        );
        let error = buffer
            .preflight_with_limits(&response(vec![encrypted_op(9)]), byte_limits)
            .expect_err("one encoded payload over the cumulative limit must fail");
        assert!(error.to_string().contains("sync_resource_limit"));
        assert_eq!(
            (
                buffer.payload_count(),
                buffer.count(),
                buffer.encoded_bytes()
            ),
            before
        );
        Ok(())
    }

    #[test]
    fn empty_payloads_consume_payload_budget_and_clear_resets_all_counters() -> Result<()> {
        let limits = PendingResourceLimits {
            payload_count: 1,
            fact_count: 1,
            encoded_bytes: 1,
        };
        let mut buffer = PendingOpsBuffer::new();
        let empty = response(Vec::new());
        let admission = buffer.preflight_with_limits(&empty, limits)?;
        buffer.push_admitted(PendingSyncPayload::Ops(empty), admission);
        assert!(
            buffer
                .preflight_with_limits(&response(Vec::new()), limits)
                .expect_err("empty frame must still consume payload budget")
                .to_string()
                .contains("sync_resource_limit")
        );

        let taken = buffer.take();
        assert!(buffer.is_empty());
        assert_eq!(
            (
                buffer.payload_count(),
                buffer.count(),
                buffer.encoded_bytes()
            ),
            (0, 0, 0)
        );
        buffer = taken;
        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(
            (
                buffer.payload_count(),
                buffer.count(),
                buffer.encoded_bytes()
            ),
            (0, 0, 0)
        );
        Ok(())
    }

    #[test]
    fn production_manual_limits_match_transfer_ceilings() {
        assert_eq!(PENDING_RESOURCE_LIMITS.payload_count, 16 * 1024);
        assert_eq!(
            PENDING_RESOURCE_LIMITS.fact_count,
            MAX_SYNC_FACTS_PER_PAYLOAD
        );
        assert_eq!(
            PENDING_RESOURCE_LIMITS.encoded_bytes,
            MAX_SYNC_FACT_BYTES_PER_PAYLOAD
        );
        assert!(
            checked_total(u64::MAX, 1, "test")
                .expect_err("counter overflow must fail closed")
                .to_string()
                .contains("sync_resource_limit")
        );
    }
}
