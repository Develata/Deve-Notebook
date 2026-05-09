// crates/core/src/ledger/manager/mod.rs
//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!   - 06_repository#repo-scope-runtime
//!
//! # RepoManager 实现模块
//!
//! 将 `RepoManager` 的方法按功能域拆分为子模块。

pub mod core;
mod core_dirs;
mod core_docs_fallback;
mod core_local_registry;
mod core_mount;
pub mod locator;
pub mod types;
mod workspace;

mod authority_storage_runtime;
mod commit_apply;
mod commit_ops;
mod commit_plan;
mod commit_structure_plan;
mod dir_structure_plan;
mod dir_structure_support;
mod local_repo_metadata_repair;
mod local_repo_metadata_repair_support;
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
mod remote_repo_scan_helpers;
mod remote_repo_scan_validate;
mod remote_repo_select;
mod repair_runtime;
pub(crate) mod repo_catalog_entries;
mod repo_catalog_runtime;
mod repo_db;
mod repo_info;
mod repo_lookup;
mod repo_scope_runtime;
mod repository;
mod shadow_maintenance_runtime;
mod snapshot_ops;
mod source_control_api;
mod source_control_ops;
mod source_control_path_target;
mod source_control_query_ops;
mod source_control_runtime;
mod source_control_target;
mod source_control_target_lookup;
mod source_control_target_resolution;
mod source_control_workdir;
mod source_control_workdir_db;
mod source_control_workdir_helpers;
mod structure_ops;
pub(crate) mod structure_projection;
mod structure_projection_support;
