//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 04_storage#internal-path-normalization
//!
use crate::components::search_box::types::{
    FileOpAction, FileOpKind, InsertQuery, SearchAction, SearchResult,
};

use super::super::path_utils::{
    finalize_dst, format_arg, format_dir_arg_with_cursor, normalize_doc_path,
};

pub(super) fn build_execute_result(kind: FileOpKind, src: &str, dst: &str) -> Option<SearchResult> {
    let src_norm = normalize_doc_path(src);
    let dst_norm = finalize_dst(&src_norm, dst);

    if dst_norm.is_empty() {
        return None;
    }

    let title = match kind {
        FileOpKind::Move => format!("Move: {} -> {}", src_norm, dst_norm),
        FileOpKind::Copy => format!("Copy: {} -> {}", src_norm, dst_norm),
        FileOpKind::Remove => return None,
    };

    Some(SearchResult {
        id: format!("fileop-{}-{}", src_norm, dst_norm),
        title,
        detail: Some("FileOp".to_string()),
        score: 1.0,
        action: SearchAction::FileOp(FileOpAction {
            kind,
            src: src_norm,
            dst: Some(dst_norm),
        }),
    })
}

pub(super) fn build_insert_query(kind: &FileOpKind, src: &str, dir: &str) -> InsertQuery {
    let cmd = match kind {
        FileOpKind::Move => "mv",
        FileOpKind::Copy => "cp",
        FileOpKind::Remove => "rm",
    };
    let src_text = format_arg(src);
    let (dst_text, cursor_offset) = format_dir_arg_with_cursor(dir);
    let prefix = format!(">{} {} ", cmd, src_text);
    let query_text = format!("{}{}", prefix, dst_text);
    let cursor = prefix.len() + cursor_offset;
    InsertQuery {
        query: query_text,
        cursor,
    }
}

pub(super) fn group_header(title: &str) -> SearchResult {
    SearchResult {
        id: format!("group-{}", title.to_lowercase()),
        title: title.to_string(),
        detail: Some("Group".to_string()),
        score: 0.0,
        action: SearchAction::Noop,
    }
}

pub(super) fn error_result(msg: String) -> SearchResult {
    SearchResult {
        id: "fileop-error".to_string(),
        title: msg,
        detail: Some("Error".to_string()),
        score: 0.0,
        action: SearchAction::Noop,
    }
}
