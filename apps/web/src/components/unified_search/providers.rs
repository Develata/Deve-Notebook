use crate::components::unified_search::types::{SearchProvider, SearchResult, SearchAction};
use crate::components::command_palette::Command;
use deve_core::models::DocId;


// --- File Provider ---
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
        if query.is_empty() {
             return self.docs.iter().take(20).map(|(id, path)| {
                SearchResult {
                    id: id.to_string(),
                    title: path.clone(),
                    detail: None,
                    score: 1.0,
                    action: SearchAction::OpenDoc(*id),
                }
            }).collect();
        }

        let mut results: Vec<SearchResult> = self.docs.iter()
            .map(|(id, path)| {
                let score = sublime_fuzzy::best_match(query, path)
                    .map(|m| m.score() as f32)
                    .unwrap_or(0.0);
                (id, path, score)
            })
            .filter(|(_, _, score)| *score > 0.0)
            .map(|(id, path, score)| {
                SearchResult {
                    id: id.to_string(),
                    title: path.clone(),
                    detail: None,
                    score,
                    action: SearchAction::OpenDoc(*id),
                }
            })
            .collect();
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(20);
        results
    }

    fn execute(&self, _action: &SearchAction) {
        // Validation only, execution handled by component
    }
}

// --- Command Provider ---
pub struct CommandProvider {
    commands: Vec<Command>,
}

impl CommandProvider {
    pub fn new(commands: Vec<Command>) -> Self {
        Self { commands }
    }
}

impl SearchProvider for CommandProvider {
    fn trigger_char(&self) -> Option<char> {
        Some('>')
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let clean_query = if query.starts_with('>') { &query[1..] } else { query };
        let clean_query = clean_query.trim();

        if clean_query.is_empty() {
             return self.commands.iter().take(20).map(|cmd| {
                SearchResult {
                    id: cmd.id.clone(),
                    title: cmd.title.clone(),
                    detail: Some("Command".to_string()),
                    score: 1.0,
                    action: SearchAction::RunCommand(cmd.clone()),
                }
            }).collect();
        }

        let mut results: Vec<SearchResult> = self.commands.iter()
            .map(|cmd| {
                let score = sublime_fuzzy::best_match(clean_query, &cmd.title)
                    .map(|m| m.score() as f32)
                    .unwrap_or(0.0);
                (cmd, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .map(|(cmd, score)| {
                SearchResult {
                    id: cmd.id.clone(),
                    title: cmd.title.clone(),
                    detail: Some("Command".to_string()),
                    score,
                    action: SearchAction::RunCommand(cmd.clone()),
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(20);
        results
    }

    fn execute(&self, _action: &SearchAction) {
         // Validation only
    }
}
