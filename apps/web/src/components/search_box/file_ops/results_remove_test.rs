use super::build_remove_results;
use crate::components::search_box::types::{FileOpAction, FileOpKind, SearchAction};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

#[test]
fn remove_without_path_returns_usage_error() {
    let results = build_remove_results(&[]);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Usage: >rm <path>");
}

#[test]
fn remove_empty_path_returns_path_required_error() {
    let results = build_remove_results(&args(&["   "]));

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].title, "Path required");
}

#[test]
fn remove_normalizes_file_target_before_action() {
    let results = build_remove_results(&args(&["notes/today"]));

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
