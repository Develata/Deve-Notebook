// apps\web\src\components\search_box
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::KeyboardEvent;

use crate::components::command_palette::registry::create_static_commands;
use crate::components::search_box::file_ops;
use crate::components::search_box::providers::{self, CommandProvider, FileProvider};
use crate::components::search_box::types::{
    InsertQuery, SearchAction, SearchProvider, SearchResult,
};
use crate::hooks::use_core::CoreState;
use crate::i18n::{t, Locale};

/// 根据查询字符切换 Provider 并实时返回结果。
#[allow(clippy::too_many_arguments)]
pub fn create_results_memo(
    show: Signal<bool>,
    search_enabled: Signal<bool>,
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

        if !search_enabled.get() {
            return Vec::new();
        }

        let q = query.get();
        let current_docs = core.docs.get();
        let current_locale = locale.get();

        let is_command = q.starts_with('>');
        let is_branch = q.starts_with('@');
        let is_file_op = file_ops::detect_file_op(&q).is_some();

        if is_command {
            if is_file_op {
                let doc_list: Vec<(deve_core::models::DocId, String)> =
                    current_docs.iter().map(|(k, v)| (*k, v.clone())).collect();
                return file_ops::build_file_ops_results(&q, &doc_list, &recent_move_dirs.get());
            }
            let cmds =
                create_static_commands(current_locale, on_settings, on_open, set_show, locale);
            let provider = CommandProvider::new(cmds);
            provider.search(&q)
        } else if is_branch {
            let shadows = core.shadow_repos.get();
            let current = match core.active_branch.get() {
                None => Some("Local (Master)".to_string()),
                Some(p) => Some(p.to_string()),
            };
            let provider = providers::BranchProvider::new(shadows, current);
            provider.search(&q)
        } else if let Some(stripped) = q.strip_prefix('+') {
            // Create Mode: Only show Create option
            let path = stripped.trim();
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
        let res = providers_results.get();
        if current >= count {
            return first_selectable(&res).unwrap_or(0);
        }
        if is_selectable(res.get(current)) {
            current
        } else {
            first_selectable(&res).unwrap_or(0)
        }
    }
}

/// 构建键盘事件处理逻辑，涵盖导航与执行。
#[allow(clippy::too_many_arguments)]
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
    set_recent_move_dirs: WriteSignal<Vec<String>>,
) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    move |ev: KeyboardEvent| {
        let key = ev.key();

        // 阻止事件冒泡，防止编辑器误收按键。
        ev.stop_propagation();

        crate::shortcuts::global::handle_search_box_keydown(
            &ev,
            set_show,
            query,
            set_query,
            set_selected_index,
            input_ref,
        );

        if !show.get() {
            return;
        }

        let results = providers_results.get();
        let count = results.len();
        if count == 0 {
            return;
        }

        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                let next = next_selectable_index(&results, active_index(), 1);
                set_selected_index.set(next);
            }
            "ArrowUp" => {
                ev.prevent_default();
                let next = next_selectable_index(&results, active_index(), -1);
                set_selected_index.set(next);
            }
            "Enter" => {
                ev.prevent_default();
                let idx = active_index();
                leptos::logging::log!("Refined Debug: Key Enter. Index: {}, Count: {}", idx, count);

                if let Some(res) = results.get(idx) {
                    leptos::logging::log!(
                        "Selected Item: {} (Action: {:?})",
                        res.title,
                        res.action
                    );
                    execute_action(
                        &res.action,
                        &core,
                        set_show,
                        set_query,
                        set_selected_index,
                        input_ref,
                        set_recent_move_dirs,
                    );
                } else {
                    leptos::logging::log!("Error: Index out of bounds or item not found.");
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn execute_action(
    action: &SearchAction,
    core: &CoreState,
    set_show: WriteSignal<bool>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
    input_ref: NodeRef<leptos::html::Input>,
    set_recent_move_dirs: WriteSignal<Vec<String>>,
) {
    match action {
        SearchAction::OpenDoc(id) => {
            core.on_doc_select.run(*id);
            set_show.set(false);
        }
        SearchAction::RunCommand(cmd) => {
            cmd.action.run(());
        }
        SearchAction::SwitchBranch(branch) => {
            if branch == "Local (Master)" {
                core.on_switch_branch.run(None);
            } else {
                core.on_switch_branch.run(Some(branch.clone()));
            }
            set_show.set(false);
        }
        SearchAction::CreateDoc(path) => {
            let normalized = file_ops::normalize_doc_path(path);
            core.on_doc_create.run(normalized);
            set_show.set(false);
        }
        SearchAction::FileOp(op) => match op.kind {
            crate::components::search_box::types::FileOpKind::Move => {
                if let Some(dst) = &op.dst {
                    core.on_doc_move.run((op.src.clone(), dst.clone()));
                    update_recent_move_dirs(set_recent_move_dirs, dst);
                    set_show.set(false);
                }
            }
            crate::components::search_box::types::FileOpKind::Copy => {
                if let Some(dst) = &op.dst {
                    core.on_doc_copy.run((op.src.clone(), dst.clone()));
                    set_show.set(false);
                }
            }
            crate::components::search_box::types::FileOpKind::Remove => {
                core.on_doc_delete.run(op.src.clone());
                set_show.set(false);
            }
        },
        SearchAction::InsertQuery(InsertQuery { query, cursor }) => {
            set_query.set(query.clone());
            set_selected_index.set(0);
            let cursor = *cursor;
            request_animation_frame(move || {
                if let Some(el) = input_ref.get_untracked() {
                    let _ = el.set_selection_range(cursor as u32, cursor as u32);
                }
            });
        }
        SearchAction::Noop => {}
    }
}

fn update_recent_move_dirs(set_recent_move_dirs: WriteSignal<Vec<String>>, dst: &str) {
    let normalized = dst.replace('\\', "/");
    let parent = std::path::Path::new(&normalized)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    if parent.is_empty() {
        return;
    }
    let dir = format!("{}/", parent.replace('\\', "/"));
    set_recent_move_dirs.update(|list| {
        list.retain(|d| d != &dir);
        list.insert(0, dir);
        if list.len() > 4 {
            list.truncate(4);
        }
    });
}

pub(crate) fn is_selectable(result: Option<&SearchResult>) -> bool {
    matches!(result, Some(r) if r.action != SearchAction::Noop)
}

fn first_selectable(results: &[SearchResult]) -> Option<usize> {
    results
        .iter()
        .position(|res| res.action != SearchAction::Noop)
}

fn next_selectable_index(results: &[SearchResult], current: usize, dir: i32) -> usize {
    if results.is_empty() {
        return 0;
    }
    let mut idx = current as i32;
    for _ in 0..results.len() {
        idx = (idx + dir).rem_euclid(results.len() as i32);
        if results[idx as usize].action != SearchAction::Noop {
            return idx as usize;
        }
    }
    current
}
