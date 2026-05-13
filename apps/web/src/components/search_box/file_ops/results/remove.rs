//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use crate::components::search_box::types::{FileOpAction, FileOpKind, SearchAction, SearchResult};
use crate::i18n::{Locale, t};
use deve_core::protocol::doc_file_op_errors as path_err;

use super::super::path_utils::{normalize_doc_path, validate_doc_shell_path};

#[cfg(test)]
mod tests;

pub(super) fn build_remove_results(args: &[String], locale: Locale) -> Vec<SearchResult> {
    if args.is_empty() {
        return vec![super::error_result(
            locale,
            t::search::remove_usage(locale).to_string(),
        )];
    }
    if args.len() > 1 {
        return vec![super::error_result(
            locale,
            t::search::paths_with_spaces_must_be_quoted(locale).to_string(),
        )];
    }
    if args[0].trim().is_empty() {
        return vec![super::error_result(
            locale,
            path_err::PATH_REQUIRED.to_string(),
        )];
    }

    let path = normalize_doc_path(&args[0]);
    if let Some(err) = validate_doc_shell_path(&path) {
        return vec![super::error_result(locale, err.to_string())];
    }
    vec![SearchResult {
        id: format!("rm-{}", path),
        title: t::search::remove_file_op(locale, &path),
        detail: Some(t::search::file_op_detail(locale).to_string()),
        score: 1.0,
        action: SearchAction::FileOp(FileOpAction {
            kind: FileOpKind::Remove,
            src: path,
            dst: None,
        }),
    }]
}
