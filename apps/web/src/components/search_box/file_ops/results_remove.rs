//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use crate::components::search_box::types::{FileOpAction, FileOpKind, SearchAction, SearchResult};
use deve_core::protocol::doc_file_op_errors as path_err;

use super::super::path_utils::{normalize_doc_path, validate_doc_shell_path};

#[cfg(test)]
#[path = "results_remove_test.rs"]
mod tests;

pub(super) fn build_remove_results(args: &[String]) -> Vec<SearchResult> {
    if args.is_empty() {
        return vec![super::error_result("Usage: >rm <path>".to_string())];
    }
    if args.len() > 1 {
        return vec![super::error_result(
            "Paths with spaces must be quoted".to_string(),
        )];
    }
    if args[0].trim().is_empty() {
        return vec![super::error_result(path_err::PATH_REQUIRED.to_string())];
    }

    let path = normalize_doc_path(&args[0]);
    if let Some(err) = validate_doc_shell_path(&path) {
        return vec![super::error_result(err.to_string())];
    }
    vec![SearchResult {
        id: format!("rm-{}", path),
        title: format!("Remove: {}", path),
        detail: Some("FileOp".to_string()),
        score: 1.0,
        action: SearchAction::FileOp(FileOpAction {
            kind: FileOpKind::Remove,
            src: path,
            dst: None,
        }),
    }]
}
