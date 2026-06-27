use super::*;
use crate::components::search_box::file_ops::parser::ParsedArgs;
use crate::components::search_box::types::{FileOpAction, FileOpKind, SearchAction};
use crate::i18n::Locale;

fn parsed_args(args: &[&str], ends_with_space: bool) -> ParsedArgs {
    ParsedArgs {
        args: args.iter().map(|s| (*s).to_string()).collect(),
        in_quote: false,
        ends_with_space,
        error: None,
    }
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn docs(paths: &[&str]) -> Vec<(DocId, String)> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| (DocId::from_u128(index as u128 + 1), (*path).to_string()))
        .collect()
}

#[test]
fn move_same_source_and_destination_returns_error() {
    let parsed = parsed_args(&["notes/today.md", "notes/today.md"], false);
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &[], &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Destination must differ from source");
}

#[test]
fn move_same_directory_target_returns_error_after_finalization() {
    let parsed = parsed_args(&["notes/today.md", "notes/"], false);
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &[], &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Destination must differ from source");
}

#[test]
fn copy_different_destination_builds_file_op_action() {
    let parsed = parsed_args(&["notes/today.md", "archive/"], false);
    let docs = docs(&["notes/today.md"]);
    let results = build_move_copy_results(FileOpKind::Copy, &parsed, &docs, &[], Locale::En);

    assert_eq!(results.len(), 1);
    match &results[0].action {
        SearchAction::FileOp(FileOpAction { kind, src, dst }) => {
            assert_eq!(*kind, FileOpKind::Copy);
            assert_eq!(src, "notes/today.md");
            assert_eq!(dst.as_deref(), Some("archive/today.md"));
        }
        other => panic!("expected FileOp result, got {:?}", other),
    }
}

#[test]
fn move_non_markdown_leaf_destination_matches_backend_canonical_path() {
    let parsed = parsed_args(&["notes/today.md", "archive/today.txt"], false);
    let docs = docs(&["notes/today.md"]);
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &docs, &[], Locale::En);

    assert_eq!(results.len(), 1);
    match &results[0].action {
        SearchAction::FileOp(FileOpAction { kind, src, dst }) => {
            assert_eq!(*kind, FileOpKind::Move);
            assert_eq!(src, "notes/today.md");
            assert_eq!(dst.as_deref(), Some("archive/today.txt.md"));
        }
        other => panic!("expected FileOp result, got {:?}", other),
    }
}

#[test]
fn move_without_source_returns_error() {
    let parsed = parsed_args(&[], false);
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &[], &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Source path required");
}

#[test]
fn move_rejects_traversal_source_path() {
    let parsed = parsed_args(&["../secret.md", "notes/ok.md"], false);
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &[], &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Invalid path");
}

#[test]
fn move_rejects_directory_source_path() {
    let parsed = parsed_args(&["notes/", "archive/"], false);
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &[], &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Invalid path");
}

#[test]
fn move_rejects_source_path_missing_from_current_docs() {
    let parsed = parsed_args(&["notes/today.md", "archive/today.md"], false);
    let docs = docs(&["archive/today.md"]);
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &docs, &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Source not found: notes/today.md");
    assert_eq!(results[0].action, SearchAction::Noop);
}

#[test]
fn copy_rejects_reserved_destination_path() {
    let parsed = parsed_args(&["notes/today.md", ".notegit/copy.md"], false);
    let results = build_move_copy_results(FileOpKind::Copy, &parsed, &[], &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Reserved internal path");
}

#[test]
fn move_directory_suggestions_skip_noop_target() {
    let parsed = parsed_args(&["notes/today.md"], true);
    let docs = [
        (DocId::from_u128(1), "notes/today.md".to_string()),
        (DocId::from_u128(2), "archive/other.md".to_string()),
    ];
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &docs, &[], Locale::En);

    let queries: Vec<String> = results
        .iter()
        .filter_map(|result| match &result.action {
            SearchAction::InsertQuery(insert) => Some(insert.query.clone()),
            _ => None,
        })
        .collect();
    assert!(
        queries
            .iter()
            .all(|query| query != ">mv notes/today.md notes/")
    );
    assert!(
        queries
            .iter()
            .any(|query| query == ">mv notes/today.md archive/")
    );
}

#[test]
fn move_directory_suggestions_filter_non_empty_destination_prefix() {
    let parsed = parsed_args(&["notes/today.md", "arc"], false);
    let docs = [
        (DocId::from_u128(1), "notes/today.md".to_string()),
        (DocId::from_u128(2), "archive/other.md".to_string()),
    ];
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &docs, &[], Locale::En);

    assert!(
        matches!(
            results.first().map(|result| &result.action),
            Some(SearchAction::FileOp(_))
        ),
        "execute candidate should remain the default result"
    );
    assert!(results.iter().any(|result| {
        matches!(
            &result.action,
            SearchAction::InsertQuery(insert) if insert.query == ">mv notes/today.md archive/"
        )
    }));
}

#[test]
fn move_directory_insert_cursor_uses_browser_utf16_offset() {
    let parsed = parsed_args(&["\u{8bb0}\u{5f55}.md"], true);
    let docs = [
        (DocId::from_u128(1), "\u{8bb0}\u{5f55}.md".to_string()),
        (
            DocId::from_u128(2),
            "\u{5f52}\u{6863}/\u{5176}\u{4ed6}.md".to_string(),
        ),
    ];
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &docs, &[], Locale::En);

    let insert = results
        .iter()
        .find_map(|result| match &result.action {
            SearchAction::InsertQuery(insert)
                if insert.query == ">mv \u{8bb0}\u{5f55}.md \u{5f52}\u{6863}/" =>
            {
                Some(insert)
            }
            _ => None,
        })
        .expect("missing unicode directory completion");

    assert_eq!(insert.cursor, utf16_len(&insert.query));
}

#[test]
fn copy_quoted_directory_insert_cursor_stays_before_closing_quote_in_utf16_offset() {
    let parsed = parsed_args(&["\u{8bb0}\u{5f55}.md"], true);
    let docs = [
        (DocId::from_u128(1), "\u{8bb0}\u{5f55}.md".to_string()),
        (
            DocId::from_u128(2),
            "\u{5f52} \u{6863}/\u{5176}\u{4ed6}.md".to_string(),
        ),
    ];
    let results = build_move_copy_results(FileOpKind::Copy, &parsed, &docs, &[], Locale::En);

    let insert = results
        .iter()
        .find_map(|result| match &result.action {
            SearchAction::InsertQuery(insert)
                if insert.query == ">cp \u{8bb0}\u{5f55}.md \"\u{5f52} \u{6863}/\"" =>
            {
                Some(insert)
            }
            _ => None,
        })
        .expect("missing quoted unicode directory completion");

    assert_eq!(
        insert.cursor,
        utf16_len(">cp \u{8bb0}\u{5f55}.md \"\u{5f52} \u{6863}/")
    );
}
