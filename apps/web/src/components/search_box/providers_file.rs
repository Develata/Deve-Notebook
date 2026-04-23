use crate::components::search_box::file_ops::validate_doc_shell_path;
use crate::components::search_box::types::{SearchAction, SearchProvider, SearchResult};
use deve_core::models::DocId;

#[cfg(test)]
#[path = "providers_file/tests.rs"]
mod tests;

pub struct FileProvider {
    docs: Vec<(DocId, String)>,
}

impl FileProvider {
    pub fn new(docs: Vec<(DocId, String)>) -> Self {
        Self { docs }
    }
}

impl SearchProvider for FileProvider {
    fn trigger_char(&self) -> Option<char> {
        None
    }

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

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(20);

        let create_query = query.trim();
        if validate_doc_shell_path(create_query).is_none()
            && !results.iter().any(|r| r.title == create_query)
        {
            results.push(SearchResult {
                id: "create-doc".to_string(),
                title: format!("Create/Open '{}'", create_query),
                detail: Some("New File".to_string()),
                score: 0.1,
                action: SearchAction::CreateDoc(create_query.to_string()),
            });
        }

        results
    }

    fn execute(&self, _action: &SearchAction) {}
}
