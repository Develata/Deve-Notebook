// crates\core\src\sync
//! # 同步缓冲模块 (Sync Buffer)
//!
//! **架构作用**:
//! 管理 Manual 模式下的待处理操作队列。
//! 暂存从远端接收但尚未合并的操作。
//!
//! **核心功能清单**:
//! - `PendingOpsBuffer`: 待合并操作缓冲区。
//! - `buffer_ops`: 暂存操作。
//! - `take_all`: 取出所有操作以供合并。
//!
//! **类型**: Core MUST (核心必选)

use crate::sync::protocol::SyncResponse;

/// Manual mode buffered payload kind.
#[derive(Clone)]
pub enum PendingSyncPayload {
    Ops(SyncResponse),
    Snapshot(SyncResponse),
}

impl PendingSyncPayload {
    pub fn ops_count(&self) -> usize {
        match self {
            Self::Ops(response) | Self::Snapshot(response) => response.ops.len(),
        }
    }
}

/// 待合并操作缓冲区
#[derive(Default, Clone)]
pub struct PendingOpsBuffer {
    /// 暂存的操作响应队列
    queue: Vec<PendingSyncPayload>,
}

impl PendingOpsBuffer {
    /// 创建新的缓冲区
    pub fn new() -> Self {
        Self::default()
    }

    /// 暂存来自远端的同步响应
    pub fn push(&mut self, response: SyncResponse) {
        self.queue.push(PendingSyncPayload::Ops(response));
    }

    /// 暂存来自远端的快照响应
    pub fn push_snapshot(&mut self, response: SyncResponse) {
        self.queue.push(PendingSyncPayload::Snapshot(response));
    }

    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// 获取待合并的操作总数 (Entry 粒度)
    pub fn count(&self) -> usize {
        self.queue.iter().map(PendingSyncPayload::ops_count).sum()
    }

    /// 克隆所有待处理 payload，供确认合并时先尝试应用。
    pub fn clone_all(&self) -> Vec<PendingSyncPayload> {
        self.queue.clone()
    }

    /// 取出所有待处理的操作 (清空缓冲区)
    pub fn take_all(&mut self) -> Vec<PendingSyncPayload> {
        std::mem::take(&mut self.queue)
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}
