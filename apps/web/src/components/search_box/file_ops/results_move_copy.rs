use crate::components::search_box::types::{FileOpKind, SearchAction, SearchResult};
use deve_core::models::DocId;
use std::collections::HashSet;

use super::super::parser::{is_ready_for_dst, ParsedArgs};
use super::super::path_utils::{collect_dirs, filter_dirs, validate_doc_shell_path};
use super::results_common::{build_execute_result, build_insert_query, group_header};

#[cfg(test)]
#[path = "results_move_copy_test.rs"]
mod tests;

pub(super) fn build_move_copy_results(
    kind: FileOpKind,
    parsed: &ParsedArgs,
    docs: &[(DocId, String)],
    recent_dirs: &[String],
) -> Vec<SearchResult> {
    if parsed.args.is_empty() {
        return vec![super::error_result("Source path required".to_string())];
    }
    if parsed.args.len() > 2 {
        return vec![super::error_result(
            "Paths with spaces must be quoted".to_string(),
        )];
    }
    if parsed
        .args
        .first()
        .map(|s| s.trim().is_empty())
        .unwrap_or(false)
    {
        return vec![super::error_result("Source path required".to_string())];
    }

    if let Some(err) = validate_doc_shell_path(&parsed.args[0]) {
        return vec![super::error_result(err.to_string())];
    }
    if parsed.args.len() == 2 && !parsed.args[1].is_empty() {
        return vec![execute_result_or_error(
            kind,
            &parsed.args[0],
            &parsed.args[1],
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
    build_dir_group_results(&kind, &src, &dst_prefix, recent_dirs, &dirs)
}

fn execute_result_or_error(kind: FileOpKind, src: &str, dst: &str) -> SearchResult {
    let Some(result) = build_execute_result(kind, src, dst) else {
        return super::error_result("Destination path required".to_string());
    };
    if let SearchAction::FileOp(action) = &result.action {
        if let Some(err) = validate_doc_shell_path(&action.src)
            .or_else(|| action.dst.as_deref().and_then(validate_doc_shell_path))
        {
            return super::error_result(err.to_string());
        }
        if action.dst.as_ref() == Some(&action.src) {
            return super::error_result("Destination must differ from source".to_string());
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
        results.push(group_header("Recent"));
        results.extend(build_dir_results(kind, src, recent_filtered));
    }
    if !all_filtered.is_empty() {
        results.push(group_header("All"));
        results.extend(build_dir_results(kind, src, all_filtered));
    }
    results
}

fn build_dir_results(kind: &FileOpKind, src: &str, dirs: Vec<(String, f32)>) -> Vec<SearchResult> {
    dirs.into_iter()
        .filter(|(dir, _)| !is_same_target(kind, src, dir))
        .map(|(dir, score)| SearchResult {
            id: format!("dir-{}", dir),
            title: dir.clone(),
            detail: Some("Directory".to_string()),
            score,
            action: SearchAction::InsertQuery(build_insert_query(kind, src, &dir)),
        })
        .collect()
}

fn is_same_target(kind: &FileOpKind, src: &str, dst: &str) -> bool {
    match build_execute_result(kind.clone(), src, dst).map(|result| result.action) {
        Some(SearchAction::FileOp(action)) => action.dst.as_ref() == Some(&action.src),
        _ => false,
    }
}
