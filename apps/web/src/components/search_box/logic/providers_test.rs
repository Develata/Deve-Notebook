use super::{SearchSurfaceMode, full_text_results, search_surface_mode};
use crate::i18n::{Locale, t};

#[test]
fn unified_search_mode_routes_command_branch_file_prefixes() {
    assert_eq!(search_surface_mode(">toggle"), SearchSurfaceMode::Command);
    assert_eq!(search_surface_mode("@branch"), SearchSurfaceMode::Branch);
    assert_eq!(search_surface_mode("file.md"), SearchSurfaceMode::File);
    assert_eq!(search_surface_mode(""), SearchSurfaceMode::File);
}

#[test]
fn unified_search_mode_routes_extended_prefixes() {
    assert_eq!(
        search_surface_mode(">mv old.md new.md"),
        SearchSurfaceMode::FileOp
    );
    assert_eq!(
        search_surface_mode(">cp old.md new.md"),
        SearchSurfaceMode::FileOp
    );
    assert_eq!(search_surface_mode(">rm old.md"), SearchSurfaceMode::FileOp);
    assert_eq!(search_surface_mode("?needle"), SearchSurfaceMode::FullText);
    assert_eq!(
        search_surface_mode("+notes/new.md"),
        SearchSurfaceMode::CreateFile
    );
}

#[test]
fn unified_search_mode_exposes_stable_dom_values() {
    assert_eq!(SearchSurfaceMode::Command.as_str(), "command");
    assert_eq!(SearchSurfaceMode::Branch.as_str(), "branch");
    assert_eq!(SearchSurfaceMode::File.as_str(), "file");
    assert_eq!(SearchSurfaceMode::FileOp.as_str(), "file-op");
    assert_eq!(SearchSurfaceMode::FullText.as_str(), "full-text");
    assert_eq!(SearchSurfaceMode::CreateFile.as_str(), "create-file");
}

#[test]
fn full_text_results_parse_doc_ids() {
    let doc_id = uuid::Uuid::new_v4();
    let results = full_text_results(
        "rust",
        vec![
            (doc_id.to_string(), "notes/rust.md".into(), 1.0),
            ("broken".into(), "notes/broken.md".into(), 1.0),
        ],
        Locale::En,
    );
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "notes/rust.md");
    assert_eq!(results[0].detail.as_deref(), Some("Full-text match"));
}

#[test]
fn full_text_results_hide_until_query_is_present() {
    assert!(full_text_results("  ", vec![], Locale::En).is_empty());
}

#[test]
fn full_text_results_localize_detail() {
    let doc_id = uuid::Uuid::new_v4();
    let results = full_text_results(
        "rust",
        vec![(doc_id.to_string(), "notes/rust.md".into(), 1.0)],
        Locale::Zh,
    );
    assert_eq!(
        results[0].detail.as_deref(),
        Some(t::search::full_text_match(Locale::Zh))
    );
}
