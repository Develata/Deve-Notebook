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
pub mod changes;
pub mod commit;
pub mod context_menu;
pub mod error_notice;
pub mod history;
pub mod repositories;
pub mod status_notice;

pub mod staged_section;
pub mod unstaged_section;

use self::changes::Changes;
use self::commit::Commit;
use self::context_menu::SectionMenu;
use self::error_notice::ErrorNotice;
use self::history::History;
use self::status_notice::StatusNotice;
use crate::components::icons::*;
use crate::hooks::use_core::SourceControlContext;
use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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
    let is_remote_branch = move || core.active_branch.get().is_some();
    let has_unstaged_changes = move || !core.unstaged_changes.get().is_empty();
    let write_block = core.write_block;
    let header_bulk_busy = StoredValue::new(Arc::new(AtomicBool::new(false)));

    let header_bulk_busy_reset = header_bulk_busy;
    Effect::new(move |_| {
        let _ = core.staged_changes.get();
        let _ = core.unstaged_changes.get();
        let _ = core.notice.get();
        header_bulk_busy_reset
            .get_value()
            .store(false, Ordering::Release);
    });

    use crate::i18n::t;

    // Derived State for Active Repo Name
    // let active_repo_label = ... (Removed)

    view! {
        <div class="h-full w-full bg-sidebar flex flex-col font-sans select-none overflow-hidden text-[13px] text-primary relative">
            // Global Header
            // Global Header
            // Global Header
            <div class="flex-none h-9 flex items-center justify-between px-4 hover:bg-hover group border-b border-transparent hover:border-default relative">
                 <div class="flex items-center gap-2 overflow-hidden">
                     <span class="font-normal text-[11px] text-secondary uppercase whitespace-nowrap">
                        {move || t::source_control::title(locale.get())}
                     </span>
                 </div>

                 <div class="flex gap-1 opacity-100 relative">
                    // More Actions Button
                    <button
                        class="p-1 hover:bg-hover rounded"
                        title=move || t::sidebar::more_actions(locale.get())
                        on:click=move |e| {
                            e.stop_propagation();
                            show_menu.update(|v| *v = !*v);
                        }
                    >
                        <MoreHorizontal class="w-3.5 h-3.5" />
                    </button>

                    // Context Menu
                    <SectionMenu
                        show_menu=show_menu
                        show_repos=show_repos
                        show_changes=show_changes
                        show_graph=show_graph
                        locale=locale
                    />
                 </div>
            </div>

            // Scrollable Content
            <div class="flex-1 overflow-y-auto">

                // 1. Repositories Section
                <crate::components::sidebar::source_control::repositories::RepositoriesSection
                    expanded=expand_repos
                    visible=show_repos
                />
                <StatusNotice block=core.write_block />
                <ErrorNotice
                    notice=core.notice
                    block=core.write_block
                    clear_notice=core.clear_notice
                />
                {move || {
                    if is_remote_branch() {
                        view! {}.into_any()
                    } else {
                        view! {
                        // 2. Changes Section
                        {move || {
                            if show_changes.get() {
                            view! {
                                <div class="border-t border-default">
                                    <button
                                         class="w-full flex items-center px-1 py-0.5 hover:bg-hover text-[11px] font-bold text-primary uppercase group focus:outline-none"
                                         on:click=move |_| expand_changes.update(|b| *b = !*b)
                                    >
                                        <span class=move || if expand_changes.get() { "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform" } else { "w-4 h-4 flex items-center justify-center transition-transform" }>
                                            <ChevronRight class="w-3 h-3" />
                                        </span>
                                        <span class="flex-1 text-left">{move || t::source_control::changes(locale.get())}</span>
                                        <div class="hidden group-hover:flex items-center gap-1">
                                            <Show when=move || write_block.get().is_none() && has_unstaged_changes()>
                                            <button
                                                class="p-0.5 hover:bg-active rounded disabled:opacity-50 disabled:cursor-not-allowed"
                                                title=move || t::source_control::discard_all_changes(locale.get())
                                                disabled=move || {
                                                    !core.can_write.get() || !has_unstaged_changes()
                                                }
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    let header_bulk_busy_discard =
                                                        header_bulk_busy.get_value();
                                                    if header_bulk_busy_discard.swap(true, Ordering::AcqRel) {
                                                        return;
                                                    }
                                                    core.clear_notice.run(());
                                                    for entry in core.unstaged_changes.get_untracked() {
                                                        core.on_discard_file.run(entry);
                                                    }
                                                }
                                            >
                                                <RefreshCw class="w-3.5 h-3.5" />
                                            </button>
                                            <button
                                                class="p-0.5 hover:bg-active rounded disabled:opacity-50 disabled:cursor-not-allowed"
                                                title=move || t::source_control::stage_all_changes(locale.get())
                                                disabled=move || {
                                                    !core.can_write.get() || !has_unstaged_changes()
                                                }
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    let header_bulk_busy_stage =
                                                        header_bulk_busy.get_value();
                                                    if header_bulk_busy_stage.swap(true, Ordering::AcqRel) {
                                                        return;
                                                    }
                                                    core.clear_notice.run(());
                                                    core.on_stage_files.run(core.unstaged_changes.get_untracked());
                                                }
                                            >
                                                <Plus class="w-3.5 h-3.5" />
                                            </button>
                                            </Show>
                                        </div>
                                    </button>

                                    {move || if expand_changes.get() {
                                        view! {
                                            <div>
                                                <Commit />
                                                <Changes />
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}}

                        // 3. History Section
                        {move || if show_graph.get() {
                            view! { <History expanded=expand_history /> }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                        }
                            .into_any()
                    }
                }}

                <div class="h-8"></div>
            </div>
        </div>
    }
}
