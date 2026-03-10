// crates/core/src/ledger/manager/mod.rs
//! # RepoManager 实现模块
//!
//! 将 `RepoManager` 的方法按功能域拆分为子模块。

pub mod core;
pub mod locator;
pub mod maintenance;
pub mod types;
mod workspace;

mod commit_apply;
mod commit_ops;
mod commit_plan;
mod commit_structure_plan;
mod dir_structure_plan;
mod dir_structure_support;
mod merge_ops;
mod metadata_ops;
mod ops_ops;
mod repository;
mod snapshot_ops;
mod source_control_api;
mod source_control_ops;
mod source_control_query_ops;
mod source_control_target;
mod source_control_target_lookup;
mod source_control_workdir;
mod source_control_workdir_helpers;
mod structure_ops;
mod structure_projection;
