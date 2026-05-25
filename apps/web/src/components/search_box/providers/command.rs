//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 14_commands#command-palette-shortcuts
//!
use crate::components::command_palette::Command;
use crate::components::search_box::score::score_desc;
use crate::components::search_box::types::{SearchAction, SearchProvider, SearchResult};
use crate::i18n::{Locale, t};

pub struct CommandProvider {
    commands: Vec<Command>,
    locale: Locale,
}

impl CommandProvider {
    pub fn new(commands: Vec<Command>, locale: Locale) -> Self {
        Self { commands, locale }
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
                    detail: Some(command_result_detail(cmd, self.locale)),
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
                detail: Some(command_result_detail(cmd, self.locale)),
                score,
                action: SearchAction::RunCommand(cmd.clone()),
            })
            .collect();

        results.sort_by(|a, b| score_desc(a.score, b.score));
        results.truncate(20);
        results
    }
}

fn command_result_detail(cmd: &Command, locale: Locale) -> String {
    cmd.availability
        .reason()
        .map(str::to_string)
        .unwrap_or_else(|| t::search::command_detail(locale).to_string())
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
        Command::available(id, title, Callback::new(|_| {}))
    }

    fn unavailable_command(id: &str, title: &str, reason: &str) -> Command {
        Command::unavailable(id, title, reason, Callback::new(|_| {}))
    }

    #[test]
    fn command_provider_matches_stable_command_id_words() {
        let provider = CommandProvider::new(
            vec![command(
                "git_import_changes",
                t::command_palette::git_import_changes(Locale::Zh),
            )],
            Locale::Zh,
        );

        let results = provider.search(">git import");

        assert_eq!(
            results.first().map(|result| result.id.as_str()),
            Some("git_import_changes")
        );
    }

    #[test]
    fn command_provider_matches_command_id_even_when_title_is_localized() {
        let provider = CommandProvider::new(
            vec![command(
                "git_push_mirror",
                t::command_palette::git_push_mirror(Locale::Zh),
            )],
            Locale::Zh,
        );

        let results = provider.search(">git push");

        assert_eq!(
            results.first().map(|result| result.id.as_str()),
            Some("git_push_mirror")
        );
    }

    #[test]
    fn command_provider_exposes_unavailable_reason_as_detail() {
        let provider = CommandProvider::new(
            vec![unavailable_command(
                "establish_branch",
                "P2P: Establish Branch",
                "Unavailable: no branch creation backend",
            )],
            Locale::En,
        );

        let results = provider.search(">establish");

        assert_eq!(
            results.first().and_then(|result| result.detail.as_deref()),
            Some("Unavailable: no branch creation backend")
        );
    }
}
