//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 14_commands#command-palette-shortcuts
//!
use crate::components::command_palette::Command;
use crate::components::search_box::score::score_desc;
use crate::components::search_box::types::{
    SearchAction, SearchProvider, SearchResult, SearchResultRole,
};
use crate::i18n::{Locale, t};

#[cfg(test)]
mod tests;

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
                    role: SearchResultRole::Action,
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
                role: SearchResultRole::Action,
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
    let mut parts = Vec::new();
    if !cmd.group.is_empty() {
        parts.push(cmd.group.clone());
    }
    if let Some(shortcut) = cmd.shortcut.as_ref()
        && !shortcut.is_empty()
    {
        parts.push(shortcut.clone());
    }
    let detail = cmd.detail_text();
    let detail = if detail.is_empty() {
        t::search::command_detail(locale).to_string()
    } else {
        detail
    };
    if !detail.is_empty() {
        parts.push(detail);
    }
    parts.join(" · ")
}

fn command_match_score(query: &str, cmd: &Command) -> f32 {
    let title_score = sublime_fuzzy::best_match(query, &cmd.title)
        .map(|m| m.score() as f32)
        .unwrap_or(0.0);
    let command_id = cmd.id.replace(['_', '-', '.'], " ");
    let id_score = sublime_fuzzy::best_match(query, &command_id)
        .map(|m| m.score() as f32)
        .unwrap_or(0.0);
    let group_score = sublime_fuzzy::best_match(query, &cmd.group)
        .map(|m| m.score() as f32)
        .unwrap_or(0.0);
    let shortcut_score = cmd
        .shortcut
        .as_deref()
        .and_then(|shortcut| sublime_fuzzy::best_match(query, shortcut))
        .map(|m| m.score() as f32)
        .unwrap_or(0.0);
    let detail_score = sublime_fuzzy::best_match(query, &cmd.detail_text())
        .map(|m| m.score() as f32)
        .unwrap_or(0.0);
    title_score
        .max(id_score)
        .max(group_score)
        .max(shortcut_score)
        .max(detail_score)
}
