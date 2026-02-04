// crates/core/src/ledger/node_meta/mod.rs
//! # Node 元数据映射模块 (Node Metadata Mapping)
//!
//! 管理 NodeId <-> Path/Meta 的映射关系。

pub mod core;
pub mod migrate;
pub mod update;

pub use core::{
    create_dir_node, ensure_dir_chain, ensure_file_node, get_node_id, get_node_meta, upsert_node,
};
pub use migrate::{list_nodes, migrate_nodes_from_docs};
pub use update::{delete_path_prefix, remove_node_by_path, rename_path_prefix};

pub(crate) fn split_path(path: &str) -> (&str, &str) {
    path.rfind('/')
        .map_or(("", path), |pos| (&path[..pos], &path[pos + 1..]))
}
