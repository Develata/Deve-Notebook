// apps/web/src/components/search_box
//! # FileOps 解析与候选生成
//!
//! 提供 `>mv` / `>cp` / `>rm` 等命令的解析、路径规范化和目录候选。

use crate::components::search_box::types::{
    FileOpAction, FileOpKind, InsertQuery, SearchAction, SearchResult,
};
use deve_core::models::DocId;
use std::collections::HashSet;
use std::path::Path;

#[derive(Clone, Debug)]
struct ParsedArgs {
    args: Vec<String>,
    in_quote: bool,
    ends_with_space: bool,
    error: Option<String>,
}

pub fn detect_file_op(query: &str) -> Option<(FileOpKind, &str)> {
    let trimmed = query.trim_start();
    if !trimmed.starts_with('>') {
        return None;
    }
    let rest = trimmed[1..].trim_start();
    let (cmd, after) = split_command(rest)?;
    let kind = match cmd {
        "mv" => FileOpKind::Move,
        "cp" => FileOpKind::Copy,
        "rm" => FileOpKind::Remove,
        _ => return None,
    };
    Some((kind, after))
}

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

pub fn build_file_ops_results(
    query: &str,
    docs: &[(DocId, String)],
    recent_dirs: &[String],
) -> Vec<SearchResult> {
    let Some((kind, after_cmd)) = detect_file_op(query) else {
        return Vec::new();
    };

    let parsed = parse_args(after_cmd);
    if let Some(err) = parsed.error {
        return vec![error_result(err)];
    }

    if parsed.in_quote {
        return vec![error_result("Unclosed quote".to_string())];
    }

    match kind {
        FileOpKind::Remove => build_remove_results(&parsed.args),
        FileOpKind::Move | FileOpKind::Copy => {
            build_move_copy_results(kind, &parsed, docs, recent_dirs)
        }
    }
}

fn build_remove_results(args: &[String]) -> Vec<SearchResult> {
    if args.is_empty() {
        return vec![error_result("Usage: >rm <path>".to_string())];
    }
    if args.len() > 1 {
        return vec![error_result("Paths with spaces must be quoted".to_string())];
    }
    if args[0].trim().is_empty() {
        return vec![error_result("Path required".to_string())];
    }

    let path = normalize_doc_path(&args[0]);
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

fn build_move_copy_results(
    kind: FileOpKind,
    parsed: &ParsedArgs,
    docs: &[(DocId, String)],
    recent_dirs: &[String],
) -> Vec<SearchResult> {
    if parsed.args.len() > 2 {
        return vec![error_result("Paths with spaces must be quoted".to_string())];
    }
    if parsed
        .args
        .first()
        .map(|s| s.trim().is_empty())
        .unwrap_or(false)
    {
        return vec![error_result("Source path required".to_string())];
    }

    let mut results = Vec::new();
    if parsed.args.len() == 2 && !parsed.args[1].is_empty() {
        if let Some(action_result) = build_execute_result(kind, &parsed.args[0], &parsed.args[1]) {
            results.push(action_result);
        }
        return results;
    }

    if !is_ready_for_dst(parsed) {
        return results;
    }

    let src = parsed.args.get(0).cloned().unwrap_or_default();
    let dst_prefix = parsed.args.get(1).cloned().unwrap_or_default();
    let dirs = collect_dirs(docs);
    let recent = if kind == FileOpKind::Move {
        recent_dirs
    } else {
        &[]
    };
    results.extend(build_dir_group_results(
        &kind,
        &src,
        &dst_prefix,
        recent,
        &dirs,
    ));
    results
}

fn build_execute_result(kind: FileOpKind, src: &str, dst: &str) -> Option<SearchResult> {
    let src_norm = normalize_doc_path(src);
    let dst_norm = finalize_dst(&src_norm, dst);
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
        .map(|(dir, score)| {
            let insert = build_insert_query(kind, src, &dir);
            SearchResult {
                id: format!("dir-{}", dir),
                title: dir.clone(),
                detail: Some("Directory".to_string()),
                score,
                action: SearchAction::InsertQuery(insert),
            }
        })
        .collect()
}

fn build_insert_query(kind: &FileOpKind, src: &str, dir: &str) -> InsertQuery {
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

fn format_arg(arg: &str) -> String {
    if arg.contains(' ') {
        format!("\"{}\"", arg)
    } else {
        arg.to_string()
    }
}

fn format_dir_arg_with_cursor(dir: &str) -> (String, usize) {
    if dir.contains(' ') {
        let text = format!("\"{}\"", dir);
        (text.clone(), text.len().saturating_sub(1))
    } else {
        let text = dir.to_string();
        (text.clone(), text.len())
    }
}

fn finalize_dst(src: &str, dst_raw: &str) -> String {
    let dst_norm = dst_raw.replace('\\', "/");
    if dst_norm.ends_with('/') {
        let base = Path::new(src)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed.md");
        return format!("{}{}", dst_norm, base);
    }
    normalize_doc_path(&dst_norm)
}

fn collect_dirs(docs: &[(DocId, String)]) -> Vec<String> {
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

fn filter_dirs(dirs: &[String], query: &str) -> Vec<(String, f32)> {
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

fn parse_args(input: &str) -> ParsedArgs {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quote = !in_quote;
                if !in_quote {
                    args.push(current.clone());
                    current.clear();
                }
            }
            c if c.is_whitespace() && !in_quote => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    ParsedArgs {
        args,
        in_quote,
        ends_with_space: input
            .chars()
            .last()
            .map(|c| c.is_whitespace())
            .unwrap_or(false),
        error: None,
    }
}

fn is_ready_for_dst(parsed: &ParsedArgs) -> bool {
    if parsed.args.len() == 1 {
        return parsed.ends_with_space;
    }
    parsed.args.len() == 2
}

fn split_command(input: &str) -> Option<(&str, &str)> {
    let mut iter = input.splitn(2, |c: char| c.is_whitespace());
    let cmd = iter.next()?.trim();
    let rest = iter.next().unwrap_or("");
    Some((cmd, rest))
}

fn group_header(title: &str) -> SearchResult {
    SearchResult {
        id: format!("group-{}", title.to_lowercase()),
        title: title.to_string(),
        detail: Some("Group".to_string()),
        score: 0.0,
        action: SearchAction::Noop,
    }
}

fn error_result(msg: String) -> SearchResult {
    SearchResult {
        id: "fileop-error".to_string(),
        title: msg,
        detail: Some("Error".to_string()),
        score: 0.0,
        action: SearchAction::Noop,
    }
}
