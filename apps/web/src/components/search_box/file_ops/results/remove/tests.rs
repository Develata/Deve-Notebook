use super::build_remove_results;
use crate::components::search_box::types::{FileOpAction, FileOpKind, SearchAction};
use crate::i18n::{Locale, t};
use deve_core::models::DocId;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

fn docs(paths: &[&str]) -> Vec<(DocId, String)> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| (DocId::from_u128(index as u128 + 1), (*path).to_string()))
        .collect()
}

#[test]
fn remove_without_path_returns_usage_error() {
    let results = build_remove_results(&[], &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Usage: >rm <path>");
}

#[test]
fn remove_empty_path_returns_path_required_error() {
    let results = build_remove_results(&args(&["   "]), &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Path required");
}

#[test]
fn remove_rejects_absolute_path() {
    let results = build_remove_results(&args(&["/notes/today.md"]), &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Invalid path");
}

#[test]
fn remove_rejects_directory_path() {
    let results = build_remove_results(&args(&["notes/"]), &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Invalid path");
}

#[test]
fn remove_normalizes_file_target_before_action() {
    let docs = docs(&["notes/today.md"]);
    let results = build_remove_results(&args(&["notes/today"]), &docs, Locale::En);

    assert_eq!(results.len(), 1);
    match &results[0].action {
        SearchAction::FileOp(FileOpAction { kind, src, dst }) => {
            assert_eq!(*kind, FileOpKind::Remove);
            assert_eq!(src, "notes/today.md");
            assert_eq!(dst, &None);
        }
        other => panic!("expected FileOp result, got {:?}", other),
    }
}

#[test]
fn remove_rejects_missing_source_path() {
    let results = build_remove_results(&args(&["notes/missing.md"]), &[], Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Source not found: notes/missing.md");
    assert_eq!(results[0].action, SearchAction::Noop);
}

#[test]
fn remove_file_result_localizes_title_and_detail() {
    let docs = docs(&["notes/today.md"]);
    let results = build_remove_results(&args(&["notes/today"]), &docs, Locale::Zh);

    assert_eq!(
        results[0].title,
        t::search::remove_file_op(Locale::Zh, "notes/today.md")
    );
    assert_eq!(
        results[0].detail.as_deref(),
        Some(t::search::file_op_detail(Locale::Zh))
    );
}
