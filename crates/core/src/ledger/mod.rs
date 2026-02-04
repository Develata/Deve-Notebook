// crates/core/src/ledger/mod.rs
//! # 仓库管理器 (Repository Manager)
//!
//! 本模块实现 P2P Git-Flow 架构中的"三位一体隔离" (Trinity Isolation)。
//!
//! ## 架构作用
//!
//! * **Store B (Local Repo)**: 本地权威库 (`local.redb`)，只有本地操作能写入
//! * **Store C (Shadow Repos)**: 远端影子库 (`remotes/peer_X.redb`)，存储远端节点数据
//!
//! ## 模块结构
//!
//! - `schema`: 数据库表定义
//! - `init`: 初始化逻辑
//! - `metadata`: Path/DocId 映射
//! - `node_meta`: NodeId/Path/Meta 映射
//! - `node_check`: Node 表一致性检查
//! - `ops`: 操作日志读写
//! - `snapshot`: 快照管理
//! - `range`: 范围查询
//! - `shadow`: Shadow 库底层实现
//! - `shadow_manager`: Shadow DB 管理
//! - `source_control`: 版本控制集成
//! - `listing`: 文档列表
//! - `merge`: 合并引擎
//! - `manager`: RepoManager 实现分布模块

// ========== 子模块声明 ==========

pub mod database;
pub mod init;
pub mod listing;
mod manager;
pub mod merge;
pub mod metadata;
pub mod node_check;
pub mod node_meta;
pub mod ops;
pub mod range;
pub mod schema;
pub mod shadow;
mod shadow_manager;
pub mod snapshot;
pub mod source_control;
pub mod traits;

// ========== 公开导出 ==========

pub use self::schema::*;
pub use manager::types::*; // Export RepoManager and RepoInfo // Export core impl methods if they were free functions, but they are impl RepoManager
// We don't need to export manager::core because impl blocks are attached to the struct.
// But we might want to export the module for some reason? No, usually not.

#[cfg(test)]
mod tests;
