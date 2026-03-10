// crates/core/src/ledger/ops.rs
//! # 操作日志模块 (Operations Log)
//!
//! 对外保持 `crate::ledger::ops::*` 接口稳定，
//! 具体实现拆到更小的写入/查询子模块，给路线 2 的 target 重构留出空间。

#[path = "ops_query.rs"]
mod query;
#[path = "ops_write_direct.rs"]
mod write_direct;
#[path = "ops_write_generated.rs"]
mod write_generated;

pub use query::{
    count_ops_from_db, find_client_op_in_db, get_ops_from_db, get_ops_from_db_after,
    get_structure_ops_for_node_from_db, max_seq_from_db,
};
pub use write_direct::append_op_to_db;
pub use write_generated::{append_generated_client_op, append_generated_op};
