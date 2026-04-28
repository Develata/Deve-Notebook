//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 12_commands#command-palette-shortcuts
//!
use crate::components::command_palette::Command;
use crate::components::search_box::types::{SearchAction, SearchProvider, SearchResult};

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
        let clean_query = query.strip_prefix('>').unwrap_or(query).trim();

        if clean_query.is_empty() {
            return self
                .commands
                .iter()
                .take(20)
                .map(|cmd| SearchResult {
                    id: cmd.id.clone(),
                    title: cmd.title.clone(),
                    detail: Some("Command".to_string()),
                    score: 1.0,
                    action: SearchAction::RunCommand(cmd.clone()),
                })
                .collect();
        }

        let mut results: Vec<SearchResult> = self
            .commands
            .iter()
            .map(|cmd| {
                let score = sublime_fuzzy::best_match(clean_query, &cmd.title)
                    .map(|m| m.score() as f32)
                    .unwrap_or(0.0);
                (cmd, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .map(|(cmd, score)| SearchResult {
                id: cmd.id.clone(),
                title: cmd.title.clone(),
                detail: Some("Command".to_string()),
                score,
                action: SearchAction::RunCommand(cmd.clone()),
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(20);
        results
    }

    fn execute(&self, _action: &SearchAction) {}
}
