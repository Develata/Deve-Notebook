// crates/core/src/ledger/merge
//! plan_ref:
//!   - 04_storage#facts-partition
//!   - 03_rendering#document-authority-bridge
//!
// ---------------------------------------------------------------
// 模块：三路合并引擎（模块入口）
// 作用：拆分 merge 子模块并对外导出
// 功能：导出合并引擎与结果类型
// ---------------------------------------------------------------

mod diff;
mod engine;
mod region;
mod types;

pub use engine::MergeEngine;
pub use types::{ConflictHunk, MergeResult};

#[cfg(test)]
mod tests;
