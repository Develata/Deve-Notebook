// crates/core/src/ledger/ops.rs
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!
//! # 操作日志模块 (Operations Log)
//!
//! 对外保持 `crate::ledger::ops::*` 接口稳定，
//! 具体实现拆到更小的写入/查询子模块，给路线 2 的 target 重构留出空间。

use crate::models::{PeerId, RepoId};

mod query;
mod validate;
pub(crate) mod write_direct;
mod write_generated;

pub(crate) use query::get_ops_from_txn;
pub use query::{
    count_ops_from_db, find_client_op_in_db, get_ops_from_db, get_ops_from_db_after,
    get_structure_ops_for_node_from_db, max_seq_from_db,
};
pub use write_direct::append_op_to_db;
pub(crate) use write_direct::append_op_to_txn;
pub use write_generated::{append_generated_client_op, append_generated_op};

pub fn local_repo_scope(repo_name: &str) -> String {
    format!("local:{repo_name}")
}

pub fn shadow_repo_scope(peer_id: &PeerId, repo_id: &RepoId) -> String {
    format!("shadow:{peer_id}/{repo_id}")
}
