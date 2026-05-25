// crates/core/src/ledger/node_meta/mod.rs
//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/projection#projection-contract
//!
//! # Node 元数据映射模块 (Node Metadata Mapping)
//!
//! 管理 NodeId <-> Path/Meta 的映射关系。

pub mod core;
mod lookup;
pub mod migrate;
pub mod update;

pub use core::{create_dir_node, ensure_dir_chain, ensure_file_node, upsert_node};
pub use lookup::{get_node_id, get_node_meta};
pub use migrate::{file_meta_for_doc, list_file_docs, list_nodes, path_for_doc};
pub use update::{delete_path_prefix, remove_node_by_path, rename_path_prefix};

pub(crate) use core::{ensure_file_node_in_txn, upsert_node_in_txn};
pub(crate) use lookup::get_node_meta_in_txn;
pub(crate) use update::{
    delete_path_prefix_in_txn, remove_node_by_path_in_txn, rename_path_prefix_in_txn,
};

pub(crate) fn split_path(path: &str) -> (&str, &str) {
    path.rfind('/')
        .map_or(("", path), |pos| (&path[..pos], &path[pos + 1..]))
}
