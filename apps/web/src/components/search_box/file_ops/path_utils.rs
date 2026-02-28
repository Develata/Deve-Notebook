// apps/web/src/components/search_box/file_ops/path_utils.rs
//! 路径规范化、目录收集与模糊过滤

use deve_core::models::DocId;
use std::collections::HashSet;
use std::path::Path;

pub fn normalize_doc_path(raw: &str) -> String {
    let normalized = raw.replace('\\', "/");
    if normalized.ends_with('/') {
        return normalized;
    }
    if Path::new(&normalized).extension().is_some() {
        return normalized;
    }
    format!("{}.md", normalized)
}

pub(super) fn finalize_dst(src: &str, dst_raw: &str) -> String {
    // 移除光标占位符 `|` (由 build_prefill_command 生成)
    let dst_clean = dst_raw.replace('|', "");
    let dst_norm = dst_clean.replace('\\', "/");

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
        let normalized = path.replace('\\', "/");
        let mut current = Path::new(&normalized);
        while let Some(parent) = current.parent() {
            if parent.as_os_str().is_empty() {
                break;
            }
            let dir = parent.to_string_lossy().replace('\\', "/");
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
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    results
}

pub(super) fn format_arg(arg: &str) -> String {
    if arg.contains(' ') {
        format!("\"{}\"", arg)
    } else {
        arg.to_string()
    }
}

pub(super) fn format_dir_arg_with_cursor(dir: &str) -> (String, usize) {
    if dir.contains(' ') {
        let text = format!("\"{}\"", dir);
        (text.clone(), text.len().saturating_sub(1))
    } else {
        let text = dir.to_string();
        (text.clone(), text.len())
    }
}
