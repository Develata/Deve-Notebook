use super::FileProvider;
use crate::components::search_box::types::{SearchAction, SearchProvider};
use crate::i18n::{Locale, t};
use deve_core::models::DocId;

fn provider() -> FileProvider {
    FileProvider::new(
        vec![(DocId::from_u128(1), "notes/existing.md".to_string())],
        Locale::En,
    )
}

#[test]
fn file_provider_does_not_offer_create_for_blank_query() {
    let results = provider().search("   ");

    assert!(
        results
            .iter()
            .all(|result| !matches!(result.action, SearchAction::CreateDoc(_)))
    );
}

#[test]
fn file_provider_trims_create_candidate_query() {
    let results = provider().search("  notes/new  ");

    let create = results
        .iter()
        .find_map(|result| match &result.action {
            SearchAction::CreateDoc(path) => Some((result.title.clone(), path.clone())),
            _ => None,
        })
        .expect("create candidate");
    assert_eq!(create.0, "Create/Open 'notes/new.md'");
    assert_eq!(create.1, "notes/new.md");
}

#[test]
fn file_provider_canonicalizes_non_markdown_leaf_create_candidate() {
    let results = provider().search("notes/new.txt");

    let create = results
        .iter()
        .find_map(|result| match &result.action {
            SearchAction::CreateDoc(path) => Some((result.title.clone(), path.clone())),
            _ => None,
        })
        .expect("create candidate");
    assert_eq!(create.0, "Create/Open 'notes/new.txt.md'");
    assert_eq!(create.1, "notes/new.txt.md");
}

#[test]
fn file_provider_localizes_create_candidate_title() {
    let provider = FileProvider::new(
        vec![(DocId::from_u128(1), "notes/existing.md".to_string())],
        Locale::Zh,
    );
    let results = provider.search("notes/new");

    let create = results
        .iter()
        .find_map(|result| match &result.action {
            SearchAction::CreateDoc(_) => Some(result.title.clone()),
            _ => None,
        })
        .expect("create candidate");
    assert_eq!(
        create,
        t::search::create_or_open(Locale::Zh, "notes/new.md")
    );
}

#[test]
fn file_provider_does_not_offer_create_for_trimmed_existing_doc() {
    let results = provider().search("  notes/existing.md  ");

    assert!(
        results
            .iter()
            .all(|result| !matches!(result.action, SearchAction::CreateDoc(_)))
    );
}

#[test]
fn file_provider_does_not_offer_create_for_existing_doc_without_extension() {
    let results = provider().search("  notes/existing  ");

    assert!(
        results
            .iter()
            .all(|result| !matches!(result.action, SearchAction::CreateDoc(_)))
    );
}

#[test]
fn file_provider_does_not_offer_create_for_reserved_path() {
    let results = provider().search(".notegit/config");

    assert!(
        results
            .iter()
            .all(|result| !matches!(result.action, SearchAction::CreateDoc(_)))
    );
}

#[test]
fn file_provider_does_not_offer_create_for_shell_reserved_path() {
    let results = provider().search("notes/a|b.md");

    assert!(
        results
            .iter()
            .all(|result| !matches!(result.action, SearchAction::CreateDoc(_)))
    );
}
