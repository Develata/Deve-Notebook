// apps\web\src\components\search_box
// 统一搜索组件模块入口。
pub mod types;
pub mod providers;

mod effects;
mod logic;
mod ui;

use leptos::prelude::*;
use std::sync::Arc;
use crate::i18n::Locale;
use crate::hooks::use_core::CoreState;

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
    let (selected_index, set_selected_index) = signal(0);
    let input_ref = NodeRef::<leptos::html::Input>::new();

    // 打开时重置查询并聚焦输入，关闭时返回编辑器焦点。
    effects::attach_focus_effect(
        show,
        mode_signal,
        set_query,
        set_selected_index,
        input_ref,
    );

    // 按查询类型动态选择 Provider 并生成结果列表。
    let providers_results = logic::create_results_memo(
        show,
        query.into(),
        locale,
        core.clone(),
        on_settings,
        on_open,
        set_show,
    );

    let active_index = Arc::new(logic::make_active_index(selected_index.into(), providers_results));

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
    ));

    let placeholder_text = logic::create_placeholder_memo(query.into(), locale);

    // 视图层拆分到 ui 模块，保证组件主体精简。
    ui::render_overlay(
        show,
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
    )
}
