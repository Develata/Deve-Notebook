//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 12_commands#command-palette-shortcuts
//!
use crate::components::command_palette::Command;
use crate::components::search_box::score::score_desc;
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
                let score = command_match_score(clean_query, cmd);
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

        results.sort_by(|a, b| score_desc(a.score, b.score));
        results.truncate(20);
        results
    }
}

fn command_match_score(query: &str, cmd: &Command) -> f32 {
    let title_score = sublime_fuzzy::best_match(query, &cmd.title)
        .map(|m| m.score() as f32)
        .unwrap_or(0.0);
    let command_id = cmd.id.replace(['_', '-', '.'], " ");
    let id_score = sublime_fuzzy::best_match(query, &command_id)
        .map(|m| m.score() as f32)
        .unwrap_or(0.0);
    title_score.max(id_score)
}

#[cfg(test)]
mod tests {
    use super::CommandProvider;
    use crate::components::command_palette::Command;
    use crate::components::search_box::types::SearchProvider;
    use crate::i18n::{Locale, t};
    use leptos::prelude::Callback;

    fn command(id: &str, title: &str) -> Command {
        Command {
            id: id.to_string(),
            title: title.to_string(),
            action: Callback::new(|_| {}),
            is_file: false,
        }
    }

    #[test]
    fn command_provider_matches_stable_command_id_words() {
        let provider = CommandProvider::new(vec![command(
            "git_import_changes",
            t::command_palette::git_import_changes(Locale::Zh),
        )]);

        let results = provider.search(">git import");

        assert_eq!(
            results.first().map(|result| result.id.as_str()),
            Some("git_import_changes")
        );
    }

    #[test]
    fn command_provider_matches_command_id_even_when_title_is_localized() {
        let provider = CommandProvider::new(vec![command(
            "git_push_mirror",
            t::command_palette::git_push_mirror(Locale::Zh),
        )]);

        let results = provider.search(">git push");

        assert_eq!(
            results.first().map(|result| result.id.as_str()),
            Some("git_push_mirror")
        );
    }
}
