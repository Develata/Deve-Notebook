//! # 核心数据模型
//!
//! 本模块定义了 Deve-Note 中使用的基础数据类型。
//!
//! ## 类型说明
//!
//! - `DocId`: 文档唯一标识符（基于 UUID）
//! - `Op`: 编辑操作（插入 Insert 或 删除 Delete）
//! - `LedgerEntry`: 带时间戳的操作记录，用于持久化
//! - `FileNodeId`: 跨平台文件系统标识符（inode/file index）

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fmt;

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
