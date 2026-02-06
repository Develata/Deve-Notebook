use crate::components::command_palette::registry::create_static_commands;
use crate::components::search_box::file_ops;
use crate::components::search_box::providers::{self, CommandProvider, FileProvider};
use crate::components::search_box::types::{SearchProvider, SearchResult};
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[allow(clippy::too_many_arguments)]
pub fn create_results_memo(
    show: Signal<bool>,
    query: Signal<String>,
    locale: RwSignal<Locale>,
    core: CoreState,
    recent_move_dirs: Signal<Vec<String>>,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    set_show: WriteSignal<bool>,
) -> Memo<Vec<SearchResult>> {
    Memo::new(move |_| {
        if !show.get() {
            return Vec::new();
        }
        let q = query.get();
        let docs = core.docs.get();
        let now_locale = locale.get();

        if q.starts_with('>') {
            if file_ops::detect_file_op(&q).is_some() {
                let doc_list = docs
                    .iter()
                    .map(|(k, v)| (*k, v.clone()))
                    .collect::<Vec<_>>();
                return file_ops::build_file_ops_results(&q, &doc_list, &recent_move_dirs.get());
            }
            let cmds = create_static_commands(now_locale, on_settings, on_open, set_show, locale);
            return CommandProvider::new(cmds).search(&q);
        }
        if q.starts_with('@') {
            let current = core
                .active_branch
                .get()
                .map(|p| p.to_string())
                .or(Some("Local (Master)".to_string()));
            return providers::BranchProvider::new(core.shadow_repos.get(), current).search(&q);
        }
        if let Some(stripped) = q.strip_prefix('+') {
            let path = stripped.trim();
            return if path.is_empty() {
                Vec::new()
            } else {
                vec![SearchResult {
                    id: "create-doc-only".to_string(),
                    title: format!("{}: '{}'", t::common::create(now_locale), path),
                    detail: Some(t::common::new_file(now_locale).to_string()),
                    score: 1.0,
                    action: crate::components::search_box::types::SearchAction::CreateDoc(
                        path.to_string(),
                    ),
                }]
            };
        }
        let doc_list = docs
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect::<Vec<_>>();
        FileProvider::new(doc_list).search(&q)
    })
}

pub fn create_placeholder_memo(query: Signal<String>, locale: RwSignal<Locale>) -> Memo<String> {
    Memo::new(move |_| {
        let q = query.get();
        let now_locale = locale.get();
        if q.starts_with('>') {
            t::search::placeholder_command(now_locale).to_string()
        } else if q.starts_with('@') {
            t::search::placeholder_branch(now_locale).to_string()
        } else if q.starts_with('+') {
            t::common::new_file(now_locale).to_string()
        } else {
            t::search::placeholder_file(now_locale).to_string()
        }
    })
}
