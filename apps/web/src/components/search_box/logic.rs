// apps\web\src\components\search_box
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::KeyboardEvent;

use crate::components::command_palette::registry::create_static_commands;
use crate::components::search_box::providers::{self, CommandProvider, FileProvider};
use crate::components::search_box::types::{SearchAction, SearchProvider, SearchResult};
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};

/// 根据查询字符切换 Provider 并实时返回结果。
pub fn create_results_memo(
    show: Signal<bool>,
    query: Signal<String>,
    locale: RwSignal<Locale>,
    core: CoreState,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    set_show: WriteSignal<bool>,
) -> Memo<Vec<SearchResult>> {
    Memo::new(move |_| {
        if !show.get() {
            return Vec::new();
        }

        let q = query.get();
        let current_docs = core.docs.get();
        let current_locale = locale.get();

        let is_command = q.starts_with('>');
        let is_branch = q.starts_with('@');

        if is_command {
            let cmds =
                create_static_commands(current_locale, on_settings, on_open, set_show, locale);
            let provider = CommandProvider::new(cmds);
            provider.search(&q)
        } else if is_branch {
            let shadows = core.shadow_repos.get();
            let current = match core.active_repo.get() {
                None => Some("Local (Master)".to_string()),
                Some(p) => Some(p.to_string()),
            };
            let provider = providers::BranchProvider::new(shadows, current);
            provider.search(&q)
        } else if q.starts_with('+') {
            // Create Mode: Only show Create option
            let path = q[1..].trim();
            if path.is_empty() {
                Vec::new()
            } else {
                vec![SearchResult {
                    id: "create-doc-only".to_string(),
                    title: format!("{}: '{}'", t::common::create(current_locale), path),
                    detail: Some(t::common::new_file(current_locale).to_string()),
                    score: 1.0,
                    action: SearchAction::CreateDoc(path.to_string()),
                }]
            }
        } else {
            let doc_list: Vec<(deve_core::models::DocId, String)> =
                current_docs.iter().map(|(k, v)| (*k, v.clone())).collect();
            let provider = FileProvider::new(doc_list);
            provider.search(&q)
        }
    })
}

/// 根据首字符切换占位文本。
pub fn create_placeholder_memo(query: Signal<String>, locale: RwSignal<Locale>) -> Memo<String> {
    Memo::new(move |_| {
        let q = query.get();
        let current_locale = locale.get();
        if q.starts_with('>') {
            t::search::placeholder_command(current_locale).to_string()
        } else if q.starts_with('@') {
            t::search::placeholder_branch(current_locale).to_string()
        } else if q.starts_with('+') {
            t::common::new_file(current_locale).to_string()
        } else {
            t::search::placeholder_file(current_locale).to_string()
        }
    })
}

/// 在结果数量变化时确保选中索引有效。
pub fn make_active_index(
    selected_index: Signal<usize>,
    providers_results: Memo<Vec<SearchResult>>,
) -> impl Fn() -> usize + Copy + Send + Sync + 'static {
    move || {
        let count = providers_results.get().len();
        if count == 0 {
            return 0;
        }
        let current = selected_index.get();
        if current >= count { 0 } else { current }
    }
}

/// 构建键盘事件处理逻辑，涵盖导航与执行。
pub fn build_keydown_handler(
    show: Signal<bool>,
    query: Signal<String>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
    providers_results: Memo<Vec<SearchResult>>,
    active_index: Arc<dyn Fn() -> usize + Send + Sync>,
    input_ref: NodeRef<leptos::html::Input>,
    set_show: WriteSignal<bool>,
    core: CoreState,
) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    move |ev: KeyboardEvent| {
        let key = ev.key();

        // 阻止事件冒泡，防止编辑器误收按键。
        ev.stop_propagation();

        crate::shortcuts::global::handle_search_box_keydown(
            &ev,
            set_show,
            query.into(),
            set_query,
            set_selected_index,
            input_ref,
        );

        if !show.get() {
            return;
        }

        let count = providers_results.get().len();
        if count == 0 {
            return;
        }

        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                set_selected_index.update(|i| *i = (*i + 1) % count);
            }
            "ArrowUp" => {
                ev.prevent_default();
                set_selected_index.update(|i| *i = (*i + count - 1) % count);
            }
            "Enter" => {
                ev.prevent_default();
                let idx = active_index();
                leptos::logging::log!("Refined Debug: Key Enter. Index: {}, Count: {}", idx, count);

                if let Some(res) = providers_results.get().get(idx) {
                    leptos::logging::log!(
                        "Selected Item: {} (Action: {:?})",
                        res.title,
                        res.action
                    );
                    match &res.action {
                        SearchAction::OpenDoc(id) => {
                            leptos::logging::log!("Executing OpenDoc: {}", id);
                            core.on_doc_select.run(id.clone());
                            set_show.set(false);
                        }
                        SearchAction::RunCommand(cmd) => {
                            leptos::logging::log!("Executing Command: {}", cmd.title);
                            cmd.action.run(());
                        }
                        SearchAction::SwitchBranch(branch) => {
                            leptos::logging::log!("Switching Branch: {}", branch);
                            if branch == "Local (Master)" {
                                core.set_active_repo.set(None);
                            } else {
                                core.set_active_repo
                                    .set(Some(deve_core::models::PeerId(branch.clone())));
                            }
                            set_show.set(false);
                        }
                        SearchAction::CreateDoc(path) => {
                            leptos::logging::log!("Creating Doc: {}", path);
                            let normalized = deve_core::utils::path::to_forward_slash(&path);
                            let target = if normalized.ends_with(".md") {
                                normalized.clone()
                            } else {
                                format!("{}.md", normalized)
                            };

                            core.on_doc_create.run(target);
                            set_show.set(false);
                        }
                    }
                } else {
                    leptos::logging::log!("Error: Index out of bounds or item not found.");
                }
            }
            _ => {}
        }
    }
}
