use super::{SearchSurfaceMode, create_file_results, full_text_results, search_surface_mode};
use crate::components::search_box::types::SearchAction;
use crate::hooks::use_core::SearchHit;
use crate::i18n::{Locale, t};
use deve_core::protocol::doc_file_op_errors as path_err;

fn doc_paths(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_string()).collect()
}

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
fn create_file_mode_rejects_invalid_path_before_dispatch() {
    let results = create_file_results("../secret.md", std::iter::empty(), Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, path_err::INVALID_PATH);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].action, SearchAction::Noop);
}

#[test]
fn create_file_mode_rejects_directory_path_before_dispatch() {
    let results = create_file_results("notes/", std::iter::empty(), Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, path_err::INVALID_PATH);
    assert_eq!(results[0].detail.as_deref(), Some("Error"));
    assert_eq!(results[0].action, SearchAction::Noop);
}

#[test]
fn create_file_mode_builds_create_action_for_valid_path() {
    let results = create_file_results("notes/new", std::iter::empty(), Locale::En);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Create: 'notes/new.md'");
    assert_eq!(results[0].detail.as_deref(), Some("New File"));
    assert_eq!(
        results[0].action,
        SearchAction::CreateDoc("notes/new.md".to_string())
    );
}

#[test]
fn create_file_mode_does_not_offer_duplicate_create_action() {
    let docs = doc_paths(&["notes/existing.md"]);
    let results = create_file_results(
        "notes/existing",
        docs.iter().map(String::as_str),
        Locale::En,
    );

    assert!(results.is_empty());
}

#[test]
fn full_text_results_parse_doc_ids() {
    let doc_id = uuid::Uuid::new_v4();
    let results = full_text_results(
        "rust",
        vec![
            SearchHit::new(doc_id.to_string(), "notes/rust.md".into(), 1.0),
            SearchHit::new("broken".into(), "notes/broken.md".into(), 1.0),
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
        vec![SearchHit::new(
            doc_id.to_string(),
            "notes/rust.md".into(),
            1.0,
        )],
        Locale::Zh,
    );
    assert_eq!(
        results[0].detail.as_deref(),
        Some(t::search::full_text_match(Locale::Zh))
    );
}
