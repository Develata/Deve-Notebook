//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 03_storage#internal-path-normalization
//!
use crate::components::search_box::file_ops::validate_doc_shell_path;
use crate::components::search_box::score::score_desc;
use crate::components::search_box::types::{SearchAction, SearchProvider, SearchResult};
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
                score,
                action: SearchAction::OpenDoc(*id),
            })
            .collect();

        results.sort_by(|a, b| score_desc(a.score, b.score));
        results.truncate(20);

        let create_query = query.trim();
        if validate_doc_shell_path(create_query).is_none()
            && !results.iter().any(|r| r.title == create_query)
        {
            results.push(SearchResult {
                id: "create-doc".to_string(),
                title: t::search::create_or_open(self.locale, create_query),
                detail: Some(t::common::new_file(self.locale).to_string()),
                score: 0.1,
                action: SearchAction::CreateDoc(create_query.to_string()),
            });
        }

        results
    }
}
