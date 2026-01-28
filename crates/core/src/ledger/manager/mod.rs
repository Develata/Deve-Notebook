// crates/core/src/ledger/manager/mod.rs
//! # RepoManager 实现模块
//!
//! 将 `RepoManager` 的方法按功能域拆分为子模块。
//!
//! ## 子模块
//! - `metadata_ops`: Path/DocId 映射操作
//! - `ops_ops`: 操作日志追加/读取
//! - `snapshot_ops`: 快照管理
//! - `source_control_ops`: 版本控制集成
//! - `merge_ops`: P2P 合并

mod merge_ops;
mod metadata_ops;
mod ops_ops;
mod snapshot_ops;
mod source_control_ops;
mod source_control_query_ops;
