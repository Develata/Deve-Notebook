// crates\core\src
//! # 核心数据模型 (Core Data Models)
//!
//! **架构作用**:
//! 定义 Deve-Note 中使用的基础数据类型，包括文档标识、操作记录、节点标识等，
//! 供 Local Repo、Shadow Repo 及 P2P 同步协议共同使用。
//!
//! **核心功能清单**:
//! - `PeerId`: P2P 网络中的节点唯一标识符。
//! - `DocId`: 文档唯一标识符（基于 UUID）。
//! - `Op`: 编辑操作（Insert / Delete）。
//! - `LedgerEntry`: 带时间戳的操作记录，用于持久化。
//! - `FileNodeId`: 跨平台文件系统标识符（inode/file index）。
//! - `VersionVector`: P2P 同步的版本向量（从 sync::vector 重新导出）。
//!
//! **类型**: Core MUST (核心必选)

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

pub use crate::sync::vector::VersionVector;

/// 节点唯一标识符 (Peer ID)
///
/// **功能**:
/// 用于在 P2P 网络中唯一标识一个远程节点。
///
/// **实现**:
/// 简单的 String 包装类型，通常为 UUID v4。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl PeerId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// 返回 Peer ID 字符串切片
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 转换为安全文件名
    ///
    /// **逻辑**:
    /// 将文件系统非法字符（如 `/`, `\` 等）替换为下划线 `_`。
    pub fn to_filename(&self) -> String {
        self.0
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
    }

    /// 生成随机 Peer ID (UUID v4)
    pub fn random() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocId(pub Uuid);

impl Default for DocId {
    fn default() -> Self {
        Self::new()
    }
}

impl DocId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_u128(v: u128) -> Self {
        Self(Uuid::from_u128(v))
    }

    pub fn as_u128(&self) -> u128 {
        self.0.as_u128()
    }
}

impl fmt::Display for DocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    Insert { pos: usize, content: String },
    Delete { pos: usize, len: usize },
}

/// 账本条目 (Ledger Entry)
///
/// **功能**:
/// 存储在 `ledger_ops` 表中的原子操作记录。
///
/// **字段**:
/// - `doc_id`: 所属文档 ID。
/// - `op`: 具体操作内容。
/// - `timestamp`: 操作产生的时间戳。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub doc_id: DocId,
    pub op: Op,
    pub timestamp: i64,
    /// Origin Peer ID (who created this op)
    pub peer_id: PeerId,
    /// Peer-specific causal sequence number (must be monotonic per peer)
    pub seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileNodeId {
    // Windows: (volume_serial_number as u64) << 64 | file_index
    // Linux/Unix: (device_id as u64) << 64 | inode
    // We combine them into a single u128 for easy storage
    pub id: u128,
}

/// 仓库 ID (UUID)
pub type RepoId = Uuid;

/// 仓库类型枚举
///
/// 用于指定操作的目标仓库,实现 Trinity Isolation 中的数据隔离。
///
/// # 变体说明
///
/// - `Local`: 本地权威库 (Store B)
/// - `Remote(PeerId, RepoId)`: 远端影子库 (Store C)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepoType {
    /// 本地权威库 (Store B)
    Local(RepoId),

    /// 远端影子库 (Store C)
    Remote(PeerId, RepoId),
}

impl RepoType {
    /// 获取 RepoId
    pub fn repo_id(&self) -> RepoId {
        match self {
            RepoType::Local(id) => *id,
            RepoType::Remote(_, id) => *id,
        }
    }
}
