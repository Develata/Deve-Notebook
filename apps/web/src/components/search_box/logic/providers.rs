//! plan_ref:
//!   - 14_tech_stack#search-baseline
//!   - 12_commands#command-palette-shortcuts
//!
use crate::components::command_palette::registry::create_static_commands;
use crate::components::search_box::file_ops;
use crate::components::search_box::providers::{
    self, CommandProvider, FileProvider, LOCAL_BRANCH_LABEL,
};
use crate::components::search_box::types::{SearchAction, SearchProvider, SearchResult};
use crate::hooks::use_core::CoreState;
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

        match search_surface_mode(&q) {
            SearchSurfaceMode::FileOp => {
                let doc_list = docs
                    .iter()
                    .map(|(k, v)| (*k, v.clone()))
                    .collect::<Vec<_>>();
                file_ops::build_file_ops_results(&q, &doc_list, &recent_move_dirs.get())
            }
            SearchSurfaceMode::Command => {
                let cmds =
                    create_static_commands(now_locale, on_settings, on_open, set_show, locale);
                CommandProvider::new(cmds).search(&q)
            }
            SearchSurfaceMode::Branch => {
                let current = core
                    .active_branch
                    .get()
                    .map(|p| p.to_string())
                    .or(Some(LOCAL_BRANCH_LABEL.to_string()));
                providers::BranchProvider::new(core.shadow_repos.get(), current).search(&q)
            }
            SearchSurfaceMode::FullText => {
                let stripped = q.strip_prefix('?').unwrap_or_default();
                full_text_results(stripped, core.search_results.get(), now_locale)
            }
            SearchSurfaceMode::CreateFile => {
                let path = q.strip_prefix('+').unwrap_or_default().trim();
                if path.is_empty() {
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
                }
            }
            SearchSurfaceMode::File => {
                let doc_list = docs
                    .iter()
                    .map(|(k, v)| (*k, v.clone()))
                    .collect::<Vec<_>>();
                FileProvider::new(doc_list).search(&q)
            }
        }
    })
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
    raw_results: Vec<(String, String, f32)>,
    locale: Locale,
) -> Vec<SearchResult> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let detail = t::search::full_text_match(locale);
    raw_results
        .into_iter()
        .filter_map(|(doc_id, path, score)| {
            let uuid = uuid::Uuid::parse_str(&doc_id).ok()?;
            Some(SearchResult {
                id: format!("full-text-{doc_id}"),
                title: path,
                detail: Some(detail.to_string()),
                score,
                action: SearchAction::OpenDoc(DocId(uuid)),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{SearchSurfaceMode, full_text_results, search_surface_mode};
    use crate::i18n::{Locale, t};

    #[test]
    fn unified_search_mode_routes_command_branch_file_prefixes() {
        assert_eq!(search_surface_mode(">toggle"), SearchSurfaceMode::Command);
        assert_eq!(search_surface_mode("@branch"), SearchSurfaceMode::Branch);
        assert_eq!(search_surface_mode("file.md"), SearchSurfaceMode::File);
        assert_eq!(search_surface_mode(""), SearchSurfaceMode::File);
    }

    #[test]
    fn unified_search_mode_routes_extended_prefixes() {
        assert_eq!(
            search_surface_mode(">mv old.md new.md"),
            SearchSurfaceMode::FileOp
        );
        assert_eq!(
            search_surface_mode(">cp old.md new.md"),
            SearchSurfaceMode::FileOp
        );
        assert_eq!(search_surface_mode(">rm old.md"), SearchSurfaceMode::FileOp);
        assert_eq!(search_surface_mode("?needle"), SearchSurfaceMode::FullText);
        assert_eq!(
            search_surface_mode("+notes/new.md"),
            SearchSurfaceMode::CreateFile
        );
    }

    #[test]
    fn unified_search_mode_exposes_stable_dom_values() {
        assert_eq!(SearchSurfaceMode::Command.as_str(), "command");
        assert_eq!(SearchSurfaceMode::Branch.as_str(), "branch");
        assert_eq!(SearchSurfaceMode::File.as_str(), "file");
        assert_eq!(SearchSurfaceMode::FileOp.as_str(), "file-op");
        assert_eq!(SearchSurfaceMode::FullText.as_str(), "full-text");
        assert_eq!(SearchSurfaceMode::CreateFile.as_str(), "create-file");
    }

    #[test]
    fn full_text_results_parse_doc_ids() {
        let doc_id = uuid::Uuid::new_v4();
        let results = full_text_results(
            "rust",
            vec![
                (doc_id.to_string(), "notes/rust.md".into(), 1.0),
                ("broken".into(), "notes/broken.md".into(), 1.0),
            ],
            Locale::En,
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "notes/rust.md");
        assert_eq!(results[0].detail.as_deref(), Some("Full-text match"));
    }

    #[test]
    fn full_text_results_hide_until_query_is_present() {
        assert!(full_text_results("  ", vec![], Locale::En).is_empty());
    }

    #[test]
    fn full_text_results_localize_detail() {
        let doc_id = uuid::Uuid::new_v4();
        let results = full_text_results(
            "rust",
            vec![(doc_id.to_string(), "notes/rust.md".into(), 1.0)],
            Locale::Zh,
        );
        assert_eq!(
            results[0].detail.as_deref(),
            Some(t::search::full_text_match(Locale::Zh))
        );
    }
}
