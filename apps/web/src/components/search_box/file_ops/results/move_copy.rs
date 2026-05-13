//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use crate::components::search_box::types::{FileOpKind, SearchAction, SearchResult};
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use deve_core::protocol::doc_file_op_errors as path_err;
use std::collections::HashSet;

use super::super::parser::{ParsedArgs, is_ready_for_dst};
use super::super::path_utils::{collect_dirs, filter_dirs, validate_doc_shell_path};
use super::common::{build_execute_result, build_insert_query, group_header};

#[cfg(test)]
mod tests;

pub(super) fn build_move_copy_results(
    kind: FileOpKind,
    parsed: &ParsedArgs,
    docs: &[(DocId, String)],
    recent_dirs: &[String],
    locale: Locale,
) -> Vec<SearchResult> {
    if parsed.args.first().is_none_or(|s| s.trim().is_empty()) {
        return vec![source_required_error(locale)];
    }
    if parsed.args.len() > 2 {
        return vec![super::error_result(
            locale,
            t::search::paths_with_spaces_must_be_quoted(locale).to_string(),
        )];
    }
    if let Some(err) = validate_doc_shell_path(&parsed.args[0]) {
        return vec![super::error_result(locale, err.to_string())];
    }
    if parsed.args.len() == 2 && !parsed.args[1].is_empty() {
        return vec![execute_result_or_error(
            kind,
            &parsed.args[0],
            &parsed.args[1],
            locale,
        )];
    }

    if !is_ready_for_dst(parsed) {
        return Vec::new();
    }

    let src = parsed.args.first().cloned().unwrap_or_default();
    let dst_prefix = parsed.args.get(1).cloned().unwrap_or_default();
    let dirs = collect_dirs(docs);
    let recent_dirs = if kind == FileOpKind::Move {
        recent_dirs
    } else {
        &[]
    };
    build_dir_group_results(&kind, &src, &dst_prefix, recent_dirs, &dirs, locale)
}

fn source_required_error(locale: Locale) -> SearchResult {
    super::error_result(locale, path_err::SOURCE_PATH_REQUIRED.to_string())
}

fn execute_result_or_error(kind: FileOpKind, src: &str, dst: &str, locale: Locale) -> SearchResult {
    let Some(result) = build_execute_result(kind, src, dst, locale) else {
        return super::error_result(locale, path_err::DESTINATION_PATH_REQUIRED.to_string());
    };
    if let SearchAction::FileOp(action) = &result.action {
        if let Some(err) = validate_doc_shell_path(&action.src)
            .or_else(|| action.dst.as_deref().and_then(validate_doc_shell_path))
        {
            return super::error_result(locale, err.to_string());
        }
        if action.dst.as_ref() == Some(&action.src) {
            return super::error_result(locale, path_err::DESTINATION_MUST_DIFFER.to_string());
        }
    }
    result
}

fn build_dir_group_results(
    kind: &FileOpKind,
    src: &str,
    dst_prefix: &str,
    recent_dirs: &[String],
    all_dirs: &[String],
    locale: Locale,
) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let recent_filtered = filter_dirs(recent_dirs, dst_prefix);
    let recent_set: HashSet<String> = recent_filtered.iter().map(|d| d.0.clone()).collect();
    let all_filtered = filter_dirs(
        &all_dirs
            .iter()
            .filter(|d| !recent_set.contains(*d))
            .cloned()
            .collect::<Vec<_>>(),
        dst_prefix,
    );

    if !recent_filtered.is_empty() {
        results.push(group_header(t::search::recent_group(locale), locale));
        results.extend(build_dir_results(kind, src, recent_filtered, locale));
    }
    if !all_filtered.is_empty() {
        results.push(group_header(t::search::all_group(locale), locale));
        results.extend(build_dir_results(kind, src, all_filtered, locale));
    }
    results
}

fn build_dir_results(
    kind: &FileOpKind,
    src: &str,
    dirs: Vec<(String, f32)>,
    locale: Locale,
) -> Vec<SearchResult> {
    dirs.into_iter()
        .filter(|(dir, _)| !is_same_target(kind, src, dir))
        .map(|(dir, score)| SearchResult {
            id: format!("dir-{}", dir),
            title: dir.clone(),
            detail: Some(t::search::directory_detail(locale).to_string()),
            score,
            action: SearchAction::InsertQuery(build_insert_query(kind, src, &dir)),
        })
        .collect()
}

fn is_same_target(kind: &FileOpKind, src: &str, dst: &str) -> bool {
    matches!(
        build_execute_result(kind.clone(), src, dst, Locale::En).map(|result| result.action),
        Some(SearchAction::FileOp(action)) if action.dst.as_ref() == Some(&action.src)
    )
}
