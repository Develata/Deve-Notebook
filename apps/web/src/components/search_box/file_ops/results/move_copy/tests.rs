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
    let results = build_move_copy_results(FileOpKind::Copy, &parsed, &[], &[], Locale::En);

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
    let results = build_move_copy_results(FileOpKind::Move, &parsed, &[], &[], Locale::En);

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
