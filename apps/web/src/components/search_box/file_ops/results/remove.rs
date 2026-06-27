//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::components::search_box::types::{
    FileOpAction, FileOpKind, SearchAction, SearchResult, SearchResultRole,
};
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use deve_core::protocol::doc_file_op_errors as path_err;

use super::super::path_utils::{normalize_doc_path, validate_doc_file_path};

#[cfg(test)]
mod tests;

pub(super) fn build_remove_results(
    args: &[String],
    docs: &[(DocId, String)],
    locale: Locale,
) -> Vec<SearchResult> {
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
    if let Some(err) = validate_doc_file_path(&path) {
        return vec![super::error_result(locale, err.to_string())];
    }
    if !source_exists(docs, &path) {
        return vec![super::error_result(
            locale,
            path_err::source_not_found(&path),
        )];
    }
    vec![SearchResult {
        id: format!("rm-{}", path),
        title: t::search::remove_file_op(locale, &path),
        detail: Some(t::search::file_op_detail(locale).to_string()),
        role: SearchResultRole::Action,
        score: 1.0,
        action: SearchAction::FileOp(FileOpAction {
            kind: FileOpKind::Remove,
            src: path,
            dst: None,
        }),
    }]
}

fn source_exists(docs: &[(DocId, String)], path: &str) -> bool {
    docs.iter()
        .any(|(_, doc_path)| normalize_doc_path(doc_path).as_str() == path)
}
