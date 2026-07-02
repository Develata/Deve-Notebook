// apps\web\src\components\search_box
//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 11_ui_design/01_web#web-layout-persistence
//!
// 统一搜索组件模块入口。
pub mod providers;
pub mod result_item;
pub mod types;

mod effects;
mod file_ops;
mod logic;
mod runtime;
mod score;
mod sheet_gesture;
mod ui;
mod ui_footer;
mod ui_sections;
mod ui_sheet;

use crate::hooks::use_core::{BranchContext, DocContext, EditorContext, SourceControlContext};
use crate::i18n::Locale;
use crate::runtime::session_client::SessionClient;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchUiMode {
    Overlay,
    Sheet,
}

/// 统一搜索组件，负责聚合命令、文件、分支搜索能力。
#[component]
pub fn UnifiedSearch(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] set_show: WriteSignal<bool>,
    #[prop(into)] mode_signal: Signal<String>,
    #[prop(into)] ui_mode: Signal<SearchUiMode>,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    source_control_context: SourceControlContext,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let runtime = runtime::SearchRuntime {
        session: expect_context::<SessionClient>(),
        document: expect_context::<DocContext>(),
        editor: expect_context::<EditorContext>(),
        branch: expect_context::<BranchContext>(),
    };

    let (query, set_query) = signal(String::new());
    let (selected_index, set_selected_index) = signal(0);
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let (recent_move_dirs, set_recent_move_dirs) = signal(Vec::<String>::new());
    let command_context =
        crate::components::command_palette::registry::StaticCommandContext::from_current_context()
            .with_source_control_context(source_control_context);

    // 打开时重置查询并聚焦输入，关闭时返回编辑器焦点。
    effects::attach_focus_effect(show, mode_signal, set_query, set_selected_index, input_ref);

    // 手动实现防抖 (100ms)，避免每次按键都触发昂贵的模糊搜索
    // 使用 Rc<RefCell<>> 因为 gloo_timers::Timeout 不实现 Send+Sync
    let (debounced_query, set_debounced_query) = signal(String::new());
    let timeout_handle: Rc<RefCell<Option<gloo_timers::callback::Timeout>>> =
        Rc::new(RefCell::new(None));

    {
        let timeout_handle = timeout_handle.clone();
        Effect::new(move |_| {
            let q = query.get();

            // 取消之前的计时器
            if let Some(handle) = timeout_handle.borrow_mut().take() {
                handle.cancel();
            }

            // 设置新的计时器，100ms 后更新 debounced_query
            let handle = gloo_timers::callback::Timeout::new(100, move || {
                set_debounced_query.set(q);
            });
            *timeout_handle.borrow_mut() = Some(handle);
        });
    }

    {
        let runtime_search = runtime.clone();
        let last_full_text_query: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        Effect::new(move |_| {
            if !show.get() {
                *last_full_text_query.borrow_mut() = None;
                return;
            }
            let q = debounced_query.get();
            let Some(stripped) = q.strip_prefix('?') else {
                *last_full_text_query.borrow_mut() = None;
                return;
            };
            let search_query = stripped.trim().to_string();
            if search_query.is_empty() {
                *last_full_text_query.borrow_mut() = None;
                return;
            }
            if last_full_text_query.borrow().as_deref() == Some(search_query.as_str()) {
                return;
            }
            *last_full_text_query.borrow_mut() = Some(search_query.clone());
            runtime_search.document.on_search.run(search_query);
        });
    }

    // 按查询类型动态选择 Provider 并生成结果列表。
    let providers_results = logic::create_results_memo(logic::SearchResultsMemoInput {
        show,
        query: debounced_query.into(),
        locale,
        runtime: runtime.clone(),
        recent_move_dirs: recent_move_dirs.into(),
        on_settings,
        on_open,
        set_show,
        command_context,
    });

    let active_index = Arc::new(logic::make_active_index(
        selected_index.into(),
        providers_results,
    ));

    // 键盘导航与执行逻辑。
    let handle_keydown = Arc::new(logic::build_keydown_handler(
        logic::SearchKeydownHandlerInput {
            show,
            query: query.into(),
            set_query,
            set_selected_index,
            providers_results,
            active_index: active_index.clone(),
            input_ref,
            set_show,
            runtime: runtime.clone(),
            set_recent_move_dirs,
        },
    ));

    let placeholder_text = logic::create_placeholder_memo(query.into(), locale);

    // 视图层拆分到 ui 模块，保证组件主体精简。
    ui::render_overlay(ui::SearchOverlayView {
        show,
        set_show,
        query: query.into(),
        set_query,
        placeholder_text,
        handle_keydown: handle_keydown.clone(),
        providers_results,
        selected_index: selected_index.into(),
        set_selected_index,
        active_index: active_index.clone(),
        input_ref,
        runtime,
        locale,
        set_recent_move_dirs,
        ui_mode,
    })
}
