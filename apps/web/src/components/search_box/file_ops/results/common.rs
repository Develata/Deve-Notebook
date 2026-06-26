//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 03_storage/index#internal-path-normalization
//!
use crate::components::search_box::types::{
    FileOpAction, FileOpKind, InsertQuery, SearchAction, SearchResult,
};
use crate::i18n::{Locale, t};

use super::super::path_utils::{
    finalize_dst, format_arg, format_dir_arg_with_cursor, normalize_doc_path, utf16_len,
};

pub(super) fn build_execute_result(
    kind: FileOpKind,
    src: &str,
    dst: &str,
    locale: Locale,
) -> Option<SearchResult> {
    let src_norm = normalize_doc_path(src);
    let dst_norm = finalize_dst(&src_norm, dst);

    if dst_norm.is_empty() {
        return None;
    }

    let title = match kind {
        FileOpKind::Move => t::search::move_file_op(locale, &src_norm, &dst_norm),
        FileOpKind::Copy => t::search::copy_file_op(locale, &src_norm, &dst_norm),
        FileOpKind::Remove => return None,
    };

    Some(SearchResult {
        id: format!("fileop-{}-{}", src_norm, dst_norm),
        title,
        detail: Some(t::search::file_op_detail(locale).to_string()),
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
    let cursor = utf16_len(&prefix) + cursor_offset;
    InsertQuery {
        query: query_text,
        cursor,
    }
}

pub(super) fn group_header(title: &str, locale: Locale) -> SearchResult {
    SearchResult {
        id: format!("group-{}", title.to_lowercase()),
        title: title.to_string(),
        detail: Some(t::search::group_detail(locale).to_string()),
        score: 0.0,
        action: SearchAction::Noop,
    }
}

pub(super) fn error_result(locale: Locale, msg: String) -> SearchResult {
    SearchResult {
        id: "fileop-error".to_string(),
        title: msg,
        detail: Some(t::search::error_detail(locale).to_string()),
        score: 0.0,
        action: SearchAction::Noop,
    }
}
