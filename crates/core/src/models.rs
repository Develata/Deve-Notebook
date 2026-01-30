// crates\core\src
//! # 核心数据模型 (Core Data Models)
//!
//! **架构作用**:
//! 定义 Deve-Note 中使用的基础数据类型，包括文档标识、操作记录、节点标识等，
//! 供 Local Repo、Shadow Repo 及 P2P 同步协议共同使用。
//!
//! **核心功能清单**:
//! - `PeerId`: P2P 网络中的节点唯一标识符 (栈分配优化)。
//! - `DocId`: 文档唯一标识符（基于 UUID）。
//! - `Op`: 编辑操作（Insert / Delete）。
//! - `LedgerEntry`: 带时间戳的操作记录，用于持久化。
//! - `FileNodeId`: 跨平台文件系统标识符（inode/file index）。
//! - `VersionVector`: P2P 同步的版本向量（从 sync::vector 重新导出）。
//!
//! **类型**: Core MUST (核心必选)

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::fmt;
use uuid::Uuid;

pub use crate::sync::vector::VersionVector;

/// 节点唯一标识符 (Peer ID)
///
/// ## 内存优化
///
/// 使用 `SmolStr` 代替 `String`，实现以下优化：
/// - **内联存储**: 23 字节以内的字符串直接存储在栈上，零堆分配。
/// - **不可变性**: `SmolStr` 是不可变的，避免意外修改。
/// - **Clone 开销**: 对于短字符串，Clone 是 O(1) 的栈拷贝。
///
/// ## 典型 PeerId 长度
///
/// - UUID v4: 36 字符 (超出内联阈值，会堆分配，但使用 Arc 共享)
/// - 短 ID (如 "local_watcher"): 13 字符 (内联存储)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PeerId(SmolStr);

impl PeerId {
    /// 创建新的 PeerId
    ///
    /// 接受任何可转换为字符串的类型。
    pub fn new(id: impl AsRef<str>) -> Self {
        Self(SmolStr::new(id.as_ref()))
    }

    /// 返回 Peer ID 字符串切片
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// 转换为安全文件名
    ///
    /// 将文件系统非法字符（如 `/`, `\` 等）替换为下划线 `_`。
    pub fn to_filename(&self) -> String {
        self.0
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
    }

    /// 生成随机 Peer ID (UUID v4)
    pub fn random() -> Self {
        Self(SmolStr::new(Uuid::new_v4().to_string()))
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 文档唯一标识符 (Doc ID)
///
/// 基于 UUID v4，实现 Copy trait 以支持高效传递。
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
///
/// ## 内存优化
///
/// `Insert` 变体使用 `SmolStr` 代替 `String`：
/// - 短插入 (< 23 字节) 零堆分配
/// - 典型用例：单字符输入、短词插入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    Insert { pos: usize, content: SmolStr },
    Delete { pos: usize, len: usize },
}

/// 账本条目 (Ledger Entry)
///
/// 存储在 `ledger_ops` 表中的原子操作记录。
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

/// 跨平台文件系统标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileNodeId {
    // Windows: (volume_serial_number as u64) << 64 | file_index
    // Linux/Unix: (device_id as u64) << 64 | inode
    pub id: u128,
}

/// 仓库 ID (UUID)
pub type RepoId = Uuid;

/// 仓库类型枚举
///
/// 用于指定操作的目标仓库，实现 Trinity Isolation 中的数据隔离。
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
