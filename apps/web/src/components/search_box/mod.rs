// apps\web\src\components\search_box
// 统一搜索组件模块入口。
pub mod providers;
pub mod result_item;
pub mod types;

mod effects;
mod file_ops;
mod logic;
mod ui;

use crate::hooks::use_core::CoreState;
use crate::i18n::Locale;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// 统一搜索组件，负责聚合命令、文件、分支搜索能力。
#[component]
pub fn UnifiedSearch(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] set_show: WriteSignal<bool>,
    #[prop(into)] mode_signal: Signal<String>,
    on_settings: Callback<()>,
    on_open: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let core = use_context::<CoreState>().expect("CoreState context");

    let (query, set_query) = signal(String::new());
    let search_enabled = Signal::derive(move || core.load_state.get() == "ready");
    let (selected_index, set_selected_index) = signal(0);
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let (recent_move_dirs, set_recent_move_dirs) = signal(Vec::<String>::new());

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

    // 按查询类型动态选择 Provider 并生成结果列表。
    let providers_results = logic::create_results_memo(
        show,
        search_enabled,
        debounced_query.into(),
        locale,
        core.clone(),
        recent_move_dirs.into(),
        on_settings,
        on_open,
        set_show,
    );

    let active_index = Arc::new(logic::make_active_index(
        selected_index.into(),
        providers_results,
    ));

    // 键盘导航与执行逻辑。
    let handle_keydown = Arc::new(logic::build_keydown_handler(
        show,
        query.into(),
        set_query,
        set_selected_index,
        providers_results,
        active_index.clone(),
        input_ref,
        set_show,
        core.clone(),
        set_recent_move_dirs,
    ));

    let placeholder_text = logic::create_placeholder_memo(query.into(), locale);

    // 视图层拆分到 ui 模块，保证组件主体精简。
    ui::render_overlay(
        show,
        search_enabled,
        set_show,
        query.into(),
        set_query,
        placeholder_text,
        handle_keydown.clone(),
        providers_results,
        selected_index.into(),
        set_selected_index,
        active_index.clone(),
        input_ref,
        core,
        locale,
        set_recent_move_dirs,
    )
}
