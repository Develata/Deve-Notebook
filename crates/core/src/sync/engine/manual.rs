// crates\core\src\sync\engine
use super::SyncEngine;
use crate::config::SyncMode;
use crate::sync::buffer::PendingSyncPayload;
use crate::sync::protocol::SyncResponse;
use anyhow::Result;

impl SyncEngine {
    /// 检查是否有待合并的操作 (Manual 模式)
    pub fn has_pending_ops(&self) -> bool {
        !self.pending_ops.is_empty()
    }

    /// 获取待合并操作的数量
    pub fn pending_ops_count(&self) -> usize {
        self.pending_ops.count()
    }

    /// 暂存从远端接收的操作 (Manual 模式)
    pub fn buffer_remote_ops(&mut self, response: SyncResponse) {
        self.pending_ops.push(response);
    }

    /// 暂存从远端接收的快照 (Manual 模式)
    pub fn buffer_remote_snapshot(&mut self, response: SyncResponse) {
        self.pending_ops.push_snapshot(response);
    }

    /// 根据当前同步模式处理增量 payload。
    pub fn receive_remote_ops(&mut self, response: SyncResponse) -> Result<u64> {
        if self.sync_mode == SyncMode::Auto {
            return self.apply_remote_ops(response);
        }
        let count = response.ops.len() as u64;
        self.buffer_remote_ops(response);
        Ok(count)
    }

    /// 根据当前同步模式处理快照 payload。
    pub fn receive_remote_snapshot(&mut self, response: SyncResponse) -> Result<u64> {
        if self.sync_mode == SyncMode::Auto {
            return self.apply_remote_snapshot(response);
        }
        let count = response.ops.len() as u64;
        self.buffer_remote_snapshot(response);
        Ok(count)
    }

    /// 合并所有待处理的操作 (Manual 模式显式触发)
    pub fn merge_pending(&mut self) -> Result<u64> {
        let mut total = 0u64;
        let pending = self.pending_ops.clone_all();

        for item in &pending {
            match item {
                PendingSyncPayload::Ops(response) => self.validate_remote_ops(response)?,
                PendingSyncPayload::Snapshot(response) => {
                    self.validate_remote_snapshot(response)?;
                }
            }
        }

        for item in pending {
            match item {
                PendingSyncPayload::Ops(response) => {
                    total += response.ops.len() as u64;
                    self.apply_remote_ops(response)?;
                }
                PendingSyncPayload::Snapshot(response) => {
                    total += response.ops.len() as u64;
                    self.apply_remote_snapshot(response)?;
                }
            }
        }

        self.pending_ops.clear();
        Ok(total)
    }

    /// 清空待处理的操作 (丢弃不合并)
    pub fn clear_pending(&mut self) {
        self.pending_ops.clear();
    }
}

#[cfg(test)]
#[path = "manual_test.rs"]
mod tests;
