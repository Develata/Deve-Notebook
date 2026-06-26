// apps/web/src/components/search_box/file_ops/path_utils.rs
//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!
//! 路径规范化、目录收集与模糊过滤

use deve_core::models::DocId;
use deve_core::protocol::doc_file_op_errors as path_err;
use deve_core::utils::path::{path_to_forward_slash, to_forward_slash};
use std::collections::HashSet;
use std::path::Path;

use crate::components::doc_shell_path::is_doc_shell_path_representable;
use crate::components::search_box::score::score_desc;

const MAX_DEPTH: usize = 10;

pub fn normalize_doc_path(raw: &str) -> String {
    let normalized = to_forward_slash(raw);
    if normalized.ends_with('/') {
        return normalized;
    }
    if normalized.ends_with(".md") {
        return normalized;
    }
    format!("{}.md", normalized)
}

pub fn validate_doc_shell_path(raw: &str) -> Option<&'static str> {
    let normalized = to_forward_slash(raw);
    let path = normalized.trim();
    if path.is_empty() {
        return Some(path_err::INVALID_EMPTY_PATH);
    }
    if !is_doc_shell_path_representable(path) {
        return Some(path_err::INVALID_PATH);
    }
    if path.contains("..") || path.starts_with('/') {
        return Some(path_err::INVALID_PATH);
    }
    let trimmed = path.trim_end_matches('/');
    let segments: Vec<_> = trimmed
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        return Some(path_err::INVALID_EMPTY_PATH);
    }
    if segments.len() > MAX_DEPTH {
        return Some(path_err::DEPTH_LIMIT_EXCEEDED);
    }
    let leaf_is_dir = path.ends_with('/');
    for (index, segment) in segments.iter().enumerate() {
        if *segment == ".notegit" {
            return Some(path_err::RESERVED_INTERNAL_PATH);
        }
        let is_leaf = index + 1 == segments.len();
        if segment.ends_with(".md") && (!is_leaf || leaf_is_dir) {
            return Some(path_err::MARKDOWN_DIRECTORY_FORBIDDEN);
        }
    }
    None
}

pub(super) fn finalize_dst(src: &str, dst_raw: &str) -> String {
    // 移除光标占位符 `|` (由 build_prefill_command 生成)
    let dst_clean = dst_raw.replace('|', "");
    let dst_norm = to_forward_slash(&dst_clean);

    // 如果清理后为空，返回空字符串 (无效目标)
    if dst_norm.trim().is_empty() {
        return String::new();
    }

    if dst_norm.ends_with('/') {
        let base = Path::new(src)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed.md");
        return format!("{}{}", dst_norm, base);
    }
    normalize_doc_path(&dst_norm)
}

pub(super) fn collect_dirs(docs: &[(DocId, String)]) -> Vec<String> {
    let mut dirs = HashSet::new();
    for (_, path) in docs.iter() {
        let normalized = to_forward_slash(path);
        let mut current = Path::new(&normalized);
        while let Some(parent) = current.parent() {
            if parent.as_os_str().is_empty() {
                break;
            }
            let dir = path_to_forward_slash(parent);
            dirs.insert(format!("{}/", dir));
            current = parent;
        }
    }
    let mut list: Vec<String> = dirs.into_iter().collect();
    list.sort();
    list
}

pub(super) fn filter_dirs(dirs: &[String], query: &str) -> Vec<(String, f32)> {
    if query.is_empty() {
        return dirs.iter().cloned().map(|d| (d, 1.0)).collect();
    }
    let mut results: Vec<(String, f32)> = dirs
        .iter()
        .filter_map(|dir| {
            sublime_fuzzy::best_match(query, dir).map(|m| (dir.clone(), m.score() as f32))
        })
        .filter(|(_, score)| *score > 0.0)
        .collect();
    results.sort_by(|a, b| score_desc(a.1, b.1));
    results
}

pub(super) fn format_arg(arg: &str) -> String {
    if arg.contains(' ') {
        format!("\"{}\"", arg)
    } else {
        arg.to_string()
    }
}

pub(super) fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

pub(super) fn format_dir_arg_with_cursor(dir: &str) -> (String, usize) {
    if dir.contains(' ') {
        let text = format!("\"{}\"", dir);
        let cursor = utf16_len(&text).saturating_sub(1);
        (text, cursor)
    } else {
        let text = dir.to_string();
        let cursor = utf16_len(&text);
        (text, cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn normalize_doc_path_uses_shared_forward_slash_policy() {
        assert_eq!(normalize_doc_path("folder\\note"), "folder/note.md");
        assert_eq!(normalize_doc_path("folder\\note.md"), "folder/note.md");
        assert_eq!(normalize_doc_path("folder\\"), "folder/");
    }

    #[test]
    fn normalize_doc_path_matches_backend_markdown_leaf_policy() {
        assert_eq!(normalize_doc_path("folder\\note.txt"), "folder/note.txt.md");
    }

    #[test]
    fn collect_dirs_uses_shared_forward_slash_policy() {
        let docs = vec![(DocId(Uuid::new_v4()), "folder\\nested\\note.md".to_string())];

        let dirs = collect_dirs(&docs);

        assert_eq!(
            dirs,
            vec!["folder/".to_string(), "folder/nested/".to_string()]
        );
    }

    #[test]
    fn validate_doc_shell_path_rejects_command_reserved_chars() {
        for path in [
            "notes/a|b.md",
            "notes/a\"b.md",
            "notes/a?b.md",
            "notes/a*b.md",
            "notes/a:b.md",
            "notes/a<b.md",
            "notes/a>b.md",
            "notes/a\nb.md",
        ] {
            assert_eq!(validate_doc_shell_path(path), Some(path_err::INVALID_PATH));
        }
    }
}
