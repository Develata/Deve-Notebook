// apps\web\src\components\source_control
//! # Source Control Module (代码控制模块)
//!
//! 包含用于版本控制的所有组件。
//! 采用 VS Code 风格的紧凑布局。
//!
//! 样式更新 (Refined):
//! - Header: "源代码管理"
//! - Section: "存储库" (Repositories) - Only shows active repo
//! - Section: "更改" (Changes)
//!
pub mod change_item;
pub mod change_item_actions;
pub mod change_item_conflict_actions;
pub mod change_item_counterpart;
pub mod change_item_meta;
pub mod change_item_workspace_actions;
pub mod changes;
pub mod changes_panel;
pub mod commit;
pub mod commit_actions;
pub mod commit_ai;
pub mod commit_controller;
pub mod commit_message_box;
pub mod context_menu;
pub mod error_notice;
pub mod header;
pub mod history;
pub mod history_commit_item;
pub mod history_compare_banner;
pub mod history_compare_logic;
pub mod history_diff_row;
pub mod history_timeline;
pub mod repositories;
pub mod staged_section_actions;
pub mod status_notice;

pub mod staged_section;
pub mod unstaged_section;
pub mod unstaged_section_actions;

use self::changes_panel::ChangesPanel;
use self::error_notice::ErrorNotice;
use self::header::SourceControlHeader;
use self::history::History;
use self::status_notice::StatusNotice;
use crate::hooks::use_core::SourceControlContext;
use leptos::prelude::*;

#[component]
pub fn SourceControlView() -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<crate::i18n::Locale>>().expect("locale context");

    // Section Expansion States
    let expand_repos = RwSignal::new(true);
    let expand_changes = RwSignal::new(true);
    let expand_history = RwSignal::new(false);

    // Section Visibility (Can be toggled via menu, simplified for now)
    let show_repos = RwSignal::new(true);
    let show_changes = RwSignal::new(true);
    let show_graph = RwSignal::new(true);

    let show_menu = RwSignal::new(false);

    view! {
        <div class="h-full w-full bg-sidebar flex flex-col font-sans select-none overflow-hidden text-[13px] text-primary relative">
            <SourceControlHeader
                locale
                show_menu
                show_repos
                show_changes
                show_graph
            />

            <div class="flex-1 overflow-y-auto">
                <crate::components::sidebar::source_control::repositories::RepositoriesSection
                    expanded=expand_repos
                    visible=show_repos
                />
                <StatusNotice block=core.write_block />
                <ErrorNotice
                    notice=core.notice
                    block=core.read_block
                    clear_notice=core.clear_notice
                />
                <ChangesPanel expanded=expand_changes visible=show_changes />
                <Show when=move || show_graph.get()>
                    <History expanded=expand_history />
                </Show>

                <div class="h-8"></div>
            </div>
        </div>
    }
}
