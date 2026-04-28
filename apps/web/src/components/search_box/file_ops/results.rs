//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
//! 搜索结果构建: Remove / Move / Copy 操作的候选结果

use crate::components::search_box::types::{FileOpKind, SearchResult};
use deve_core::models::DocId;

#[path = "results_common.rs"]
mod results_common;
#[path = "results_move_copy.rs"]
mod results_move_copy;
#[path = "results_remove.rs"]
mod results_remove;

pub(super) fn error_result(msg: String) -> SearchResult {
    results_common::error_result(msg)
}

pub(super) fn build_remove_results(args: &[String]) -> Vec<SearchResult> {
    results_remove::build_remove_results(args)
}

pub(super) fn build_move_copy_results(
    kind: FileOpKind,
    parsed: &super::parser::ParsedArgs,
    docs: &[(DocId, String)],
    recent_dirs: &[String],
) -> Vec<SearchResult> {
    results_move_copy::build_move_copy_results(kind, parsed, docs, recent_dirs)
}
