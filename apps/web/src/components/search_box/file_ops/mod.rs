// apps/web/src/components/search_box/file_ops/mod.rs
//! # FileOps 解析与候选生成
//!
//! 提供 `>mv` / `>cp` / `>rm` 等命令的解析、路径规范化和目录候选。
//! 拆分为 parser / path_utils / results 三个子模块。

mod parser;
mod path_utils;
mod results;

pub use path_utils::normalize_doc_path;

use crate::components::search_box::types::{FileOpKind, SearchResult};
use deve_core::models::DocId;

use parser::{parse_args, split_command};
use results::{build_move_copy_results, build_remove_results, error_result};

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
