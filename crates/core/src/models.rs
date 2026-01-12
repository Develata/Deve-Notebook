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
use uuid::Uuid;
use std::fmt;

pub use crate::sync::vector::VersionVector;

/// 节点唯一标识符 (用于 P2P 通信)
/// Peer ID for identifying remote nodes in the P2P network.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl PeerId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    /// Returns the peer ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Converts to a safe filename (replaces invalid characters).
    pub fn to_filename(&self) -> String {
        self.0.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocId(pub Uuid);

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    Insert { pos: usize, content: String },
    Delete { pos: usize, len: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub doc_id: DocId, // We need to know which doc this Op belongs to!
    pub op: Op,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileNodeId {
    // Windows: (volume_serial_number as u64) << 64 | file_index
    // Linux/Unix: (device_id as u64) << 64 | inode
    // We combine them into a single u128 for easy storage
    pub id: u128,
}
