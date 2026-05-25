// apps\web\src\components\command_palette
//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! CommandPalette 组件 (CommandPalette Component)
//!
//! 一个可搜索的命令面板，用于快速执行操作（仅限命令，不包括文件搜索）。

#![allow(dead_code)] // 组件参数由 Leptos 宏使用，编译器未能识别

mod logic;
pub mod registry;
mod types;
mod ui;

pub use types::Command;

use crate::i18n::Locale;
use leptos::prelude::*;
use std::sync::Arc;

#[component]
pub fn CommandPalette(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] set_show: WriteSignal<bool>,
    on_settings: Callback<()>,
    on_open: Callback<()>, // Opens the Open Document modal
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (query, set_query) = signal(String::new());
    let (selected_index, set_selected_index) = signal(0);
    let input_ref = NodeRef::<leptos::html::Input>::new();

    logic::attach_reset_effect(show, set_query, set_selected_index);
    logic::attach_focus_restore_effect(show, input_ref);

    let filtered_commands =
        logic::create_filtered_commands_memo(query.into(), locale, on_settings, on_open, set_show);
    let active_index = Arc::new(logic::make_active_index(
        selected_index.into(),
        filtered_commands,
    ));
    let handle_keydown = Arc::new(logic::build_keydown_handler(
        filtered_commands,
        selected_index.into(),
        set_selected_index,
        set_show,
        active_index.clone(),
    ));

    ui::render_overlay(ui::CommandPaletteOverlay {
        show,
        set_show,
        query: query.into(),
        set_query,
        locale,
        filtered_commands,
        selected_index: selected_index.into(),
        set_selected_index,
        handle_keydown,
        input_ref,
    })
}
