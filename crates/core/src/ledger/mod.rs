// crates/core/src/ledger/mod.rs
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#repo-catalog-contract
//!
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
//! - `inode_index`: Watcher identity side table
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

mod append_validate;
pub mod database;
mod database_cache;
mod database_open;
pub mod doc_lookup;
pub mod init;
mod init_reuse;
pub mod inode_index;
pub mod listing;
pub(crate) mod manager;
pub mod merge;
pub mod metadata;
pub mod node_check;
pub mod node_meta;
mod node_ops;
pub mod ops;
pub mod range;
pub mod reconcile;
mod runtime_tables;
pub mod schema;
pub mod seq;
pub mod shadow;
mod shadow_binding;
mod shadow_manager;
mod shadow_transfer;
pub mod snapshot;
pub mod source_control;
pub mod traits;

// ========== 公开导出 ==========

pub use self::schema::*;
pub use manager::types::*;
pub use seq::GlobalSeq;
pub(crate) use shadow_transfer::ShadowPayload; // Export RepoManager and RepoInfo // Export core impl methods if they were free functions, but they are impl RepoManager
// We don't need to export manager::core because impl blocks are attached to the struct.
// But we might want to export the module for some reason? No, usually not.

#[cfg(test)]
mod client_op_tests;
#[cfg(test)]
mod init_node_check_fail_closed_test;
#[cfg(test)]
mod ops_query_fail_closed_test;
#[cfg(test)]
mod ops_seq_overflow_test;
#[cfg(test)]
mod ops_write_validation_test;
#[cfg(test)]
mod runtime_tables_test;
#[cfg(test)]
mod schema_version_test;
#[cfg(test)]
mod structure_write_validation_test;
#[cfg(test)]
mod tests;
