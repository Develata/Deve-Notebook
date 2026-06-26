// apps/web/src/components/search_box/file_ops/mod.rs
//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! # FileOps 解析与候选生成
//!
//! 提供 `>mv` / `>cp` / `>rm` 等命令的解析、路径规范化和目录候选。
//! 拆分为 parser / path_utils / results 三个子模块。

mod parser;
mod path_utils;
mod results;

pub use path_utils::{normalize_doc_path, validate_doc_create_path};

use crate::components::search_box::types::{FileOpKind, SearchResult};
use crate::i18n::{Locale, t};
use deve_core::models::DocId;

use parser::{ParseError, parse_args, split_command};
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
    locale: Locale,
) -> Vec<SearchResult> {
    let Some((kind, after_cmd)) = detect_file_op(query) else {
        return Vec::new();
    };

    let parsed = parse_args(after_cmd);
    if let Some(err) = parsed.error {
        return vec![error_result(locale, parse_error_message(err, locale))];
    }

    if parsed.in_quote {
        return vec![error_result(
            locale,
            t::search::unclosed_quote(locale).to_string(),
        )];
    }

    match kind {
        FileOpKind::Remove => build_remove_results(&parsed.args, locale),
        FileOpKind::Move | FileOpKind::Copy => {
            build_move_copy_results(kind, &parsed, docs, recent_dirs, locale)
        }
    }
}

fn parse_error_message(error: ParseError, locale: Locale) -> String {
    match error {
        ParseError::PathsWithSpacesMustBeQuoted => {
            t::search::paths_with_spaces_must_be_quoted(locale).to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_file_ops_results;
    use crate::components::search_box::types::SearchAction;
    use crate::i18n::Locale;

    #[test]
    fn file_op_rejects_adjacent_text_after_quoted_arg_before_action() {
        let results = build_file_ops_results(">mv \"old name.md\"new.md", &[], &[], Locale::En);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Paths with spaces must be quoted");
        assert_eq!(results[0].action, SearchAction::Noop);
    }
}
