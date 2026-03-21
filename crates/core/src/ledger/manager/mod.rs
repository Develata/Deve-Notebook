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
mod local_repo_metadata_repair;
mod local_repo_names;
mod local_repo_source_control_repair;
mod merge_ops;
mod metadata_ops;
mod metadata_repair_ops;
mod ops_ops;
mod ops_structure;
mod projection_cleanup;
mod remote_repo_allocate;
mod remote_repo_scan;
mod remote_repo_scan_entry;
pub(crate) mod repo_catalog_entries;
mod repo_db;
mod repo_info;
mod repo_lookup;
mod repository;
mod snapshot_ops;
mod source_control_api;
mod source_control_ops;
mod source_control_path_target;
mod source_control_query_ops;
mod source_control_target;
mod source_control_target_lookup;
mod source_control_workdir;
mod source_control_workdir_db;
mod source_control_workdir_helpers;
mod structure_ops;
pub(crate) mod structure_projection;
mod structure_projection_support;
