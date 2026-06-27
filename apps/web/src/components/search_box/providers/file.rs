//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 03_storage/index#internal-path-normalization
//!
use crate::components::search_box::file_ops::{normalize_doc_path, validate_doc_create_path};
use crate::components::search_box::score::score_desc;
use crate::components::search_box::types::{
    SearchAction, SearchProvider, SearchResult, SearchResultRole,
};
use crate::i18n::{Locale, t};
use deve_core::models::DocId;

#[cfg(test)]
mod tests;

pub struct FileProvider {
    docs: Vec<(DocId, String)>,
    locale: Locale,
}

impl FileProvider {
    pub fn new(docs: Vec<(DocId, String)>, locale: Locale) -> Self {
        Self { docs, locale }
    }
}

impl SearchProvider for FileProvider {
    fn search(&self, query: &str) -> Vec<SearchResult> {
        let mut seen_paths = std::collections::HashSet::new();

        if query.is_empty() {
            return self
                .docs
                .iter()
                .filter(|(_, path)| seen_paths.insert(path.clone()))
                .take(20)
                .map(|(id, path)| SearchResult {
                    id: id.to_string(),
                    title: path.clone(),
                    detail: None,
                    role: SearchResultRole::Action,
                    score: 1.0,
                    action: SearchAction::OpenDoc(*id),
                })
                .collect();
        }

        let mut results: Vec<SearchResult> = self
            .docs
            .iter()
            .map(|(id, path)| {
                let score = sublime_fuzzy::best_match(query, path)
                    .map(|m| m.score() as f32)
                    .unwrap_or(0.0);
                (id, path, score)
            })
            .filter(|(_, _, score)| *score > 0.0)
            .filter(|(_, path, _)| seen_paths.insert((*path).clone()))
            .map(|(id, path, score)| SearchResult {
                id: id.to_string(),
                title: path.clone(),
                detail: None,
                role: SearchResultRole::Action,
                score,
                action: SearchAction::OpenDoc(*id),
            })
            .collect();

        results.sort_by(|a, b| score_desc(a.score, b.score));
        results.truncate(20);

        let create_query = query.trim();
        if validate_doc_create_path(create_query).is_none() {
            let create_path = normalize_doc_path(create_query);
            if self.docs.iter().any(|(_, path)| path == &create_path) {
                return results;
            }
            results.push(SearchResult {
                id: "create-doc".to_string(),
                title: t::search::create_or_open(self.locale, &create_path),
                detail: Some(t::common::new_file(self.locale).to_string()),
                role: SearchResultRole::Action,
                score: 0.1,
                action: SearchAction::CreateDoc(create_path),
            });
        }

        results
    }
}
