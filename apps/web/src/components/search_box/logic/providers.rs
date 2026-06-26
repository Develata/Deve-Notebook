//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 14_commands#command-palette-shortcuts
//!
use crate::components::command_palette::registry::create_static_commands;
use crate::components::search_box::file_ops;
use crate::components::search_box::providers::{
    self, CommandProvider, FileProvider, LOCAL_BRANCH_LABEL,
};
use crate::components::search_box::runtime::SearchRuntime;
use crate::components::search_box::types::{SearchAction, SearchProvider, SearchResult};
use crate::hooks::use_core::SearchHit;
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchSurfaceMode {
    Command,
    FileOp,
    Branch,
    FullText,
    CreateFile,
    File,
}

impl SearchSurfaceMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SearchSurfaceMode::Command => "command",
            SearchSurfaceMode::FileOp => "file-op",
            SearchSurfaceMode::Branch => "branch",
            SearchSurfaceMode::FullText => "full-text",
            SearchSurfaceMode::CreateFile => "create-file",
            SearchSurfaceMode::File => "file",
        }
    }
}

pub(crate) fn search_surface_mode(query: &str) -> SearchSurfaceMode {
    if query.starts_with('>') {
        if file_ops::detect_file_op(query).is_some() {
            SearchSurfaceMode::FileOp
        } else {
            SearchSurfaceMode::Command
        }
    } else if query.starts_with('@') {
        SearchSurfaceMode::Branch
    } else if query.starts_with('?') {
        SearchSurfaceMode::FullText
    } else if query.starts_with('+') {
        SearchSurfaceMode::CreateFile
    } else {
        SearchSurfaceMode::File
    }
}

pub struct SearchResultsMemoInput {
    pub show: Signal<bool>,
    pub query: Signal<String>,
    pub locale: RwSignal<Locale>,
    pub runtime: SearchRuntime,
    pub recent_move_dirs: Signal<Vec<String>>,
    pub on_settings: Callback<()>,
    pub on_open: Callback<()>,
    pub set_show: WriteSignal<bool>,
}

pub fn create_results_memo(input: SearchResultsMemoInput) -> Memo<Vec<SearchResult>> {
    let SearchResultsMemoInput {
        show,
        query,
        locale,
        runtime,
        recent_move_dirs,
        on_settings,
        on_open,
        set_show,
    } = input;
    Memo::new(move |_| {
        if !show.get() {
            return Vec::new();
        }
        let q = query.get();
        let docs = runtime.document.docs.get();
        let now_locale = locale.get();

        match search_surface_mode(&q) {
            SearchSurfaceMode::FileOp => {
                let doc_list = docs
                    .iter()
                    .map(|(k, v)| (*k, v.clone()))
                    .collect::<Vec<_>>();
                file_ops::build_file_ops_results(&q, &doc_list, &recent_move_dirs.get(), now_locale)
            }
            SearchSurfaceMode::Command => {
                let cmds =
                    create_static_commands(now_locale, on_settings, on_open, set_show, locale);
                CommandProvider::new(cmds, now_locale).search(&q)
            }
            SearchSurfaceMode::Branch => {
                let current = runtime
                    .branch
                    .active_branch
                    .get()
                    .map(|p| p.to_string())
                    .or(Some(LOCAL_BRANCH_LABEL.to_string()));
                providers::BranchProvider::new(
                    runtime.branch.shadow_repos.get(),
                    current,
                    now_locale,
                )
                .search(&q)
            }
            SearchSurfaceMode::FullText => {
                let stripped = q.strip_prefix('?').unwrap_or_default();
                full_text_results(stripped, runtime.document.search_results.get(), now_locale)
            }
            SearchSurfaceMode::CreateFile => {
                let path = q.strip_prefix('+').unwrap_or_default().trim();
                create_file_results(path, now_locale)
            }
            SearchSurfaceMode::File => {
                let doc_list = docs
                    .iter()
                    .map(|(k, v)| (*k, v.clone()))
                    .collect::<Vec<_>>();
                FileProvider::new(doc_list, now_locale).search(&q)
            }
        }
    })
}

fn create_file_results(path: &str, locale: Locale) -> Vec<SearchResult> {
    if path.is_empty() {
        return Vec::new();
    }
    if let Some(err) = file_ops::validate_doc_shell_path(path) {
        return vec![SearchResult {
            id: "create-doc-error".to_string(),
            title: err.to_string(),
            detail: Some(t::search::error_detail(locale).to_string()),
            score: 0.0,
            action: SearchAction::Noop,
        }];
    }
    vec![SearchResult {
        id: "create-doc-only".to_string(),
        title: format!("{}: '{}'", t::common::create(locale), path),
        detail: Some(t::common::new_file(locale).to_string()),
        score: 1.0,
        action: SearchAction::CreateDoc(path.to_string()),
    }]
}

pub fn create_placeholder_memo(query: Signal<String>, locale: RwSignal<Locale>) -> Memo<String> {
    Memo::new(move |_| {
        let q = query.get();
        let now_locale = locale.get();
        match search_surface_mode(&q) {
            SearchSurfaceMode::Command | SearchSurfaceMode::FileOp => {
                t::search::placeholder_command(now_locale).to_string()
            }
            SearchSurfaceMode::Branch => t::search::placeholder_branch(now_locale).to_string(),
            SearchSurfaceMode::FullText => t::search::placeholder_full_text(now_locale).to_string(),
            SearchSurfaceMode::CreateFile => t::common::new_file(now_locale).to_string(),
            SearchSurfaceMode::File => t::search::placeholder_file(now_locale).to_string(),
        }
    })
}

fn full_text_results(
    query: &str,
    raw_results: Vec<SearchHit>,
    locale: Locale,
) -> Vec<SearchResult> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let detail = t::search::full_text_match(locale);
    raw_results
        .into_iter()
        .filter_map(|hit| {
            let uuid = uuid::Uuid::parse_str(&hit.doc_id).ok()?;
            Some(SearchResult {
                id: format!("full-text-{}", hit.doc_id),
                title: hit.path,
                detail: Some(detail.to_string()),
                score: hit.score,
                action: SearchAction::OpenDoc(DocId(uuid)),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
