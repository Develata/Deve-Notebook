// crates\core\src\ledger
// ---------------------------------------------------------------
// 模块：三路合并引擎（模块入口）
// 作用：拆分 merge 子模块并对外导出
// 功能：导出合并引擎与结果类型
// ---------------------------------------------------------------

#[path = "merge/diff.rs"]
mod diff;
#[path = "merge/engine.rs"]
mod engine;
#[path = "merge/types.rs"]
mod types;

pub use engine::MergeEngine;
pub use types::{ConflictHunk, MergeResult};

#[cfg(test)]
#[path = "merge/tests.rs"]
mod tests;
