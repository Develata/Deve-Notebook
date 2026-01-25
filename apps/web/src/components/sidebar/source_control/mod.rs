// apps\web\src\components\source_control
//! # Source Control Module (代码控制模块)
//!
//! 包含用于版本控制的所有组件。
//! 采用 VS Code 风格的紧凑布局。
//!
//! 样式更新 (Refined):
//! - Header: "源代码管理: <ActiveRepo>"
//! - Dark/Light mode compatible classes
//! - Switch Repo via Header Dropdown

pub mod change_item;
pub mod changes;
pub mod commit;
pub mod history;

pub mod staged_section;
pub mod unstaged_section;

use self::changes::Changes;
use self::commit::Commit;
use self::history::History;
use deve_core::models::PeerId;
use leptos::prelude::*;

#[component]
pub fn SourceControlView() -> impl IntoView {
    let locale = use_context::<RwSignal<crate::i18n::Locale>>().expect("locale context");
    let core = use_context::<crate::hooks::use_core::CoreState>().expect("CoreState missing");

    let expand_changes = RwSignal::new(true);
    let expand_history = RwSignal::new(false);

    // Section Visibility States
    let show_changes = RwSignal::new(true);
    let show_graph = RwSignal::new(true);

    // Menu States
    let show_menu = RwSignal::new(false);
    let show_repo_switcher = RwSignal::new(false);

    use crate::i18n::t;

    // Derived State for Active Repo Name
    let active_repo_label = Signal::derive(move || {
        core.active_repo
            .get()
            .map(|id| id.0)
            .unwrap_or_else(|| "default.redb".to_string())
    });

    view! {
        <div class="h-full w-full bg-[#f3f3f3] dark:bg-[#252526] flex flex-col font-sans select-none overflow-hidden text-[13px] text-[#3b3b3b] dark:text-[#cccccc] relative">
            // Global Header
            <div class="flex-none h-9 flex items-center justify-between px-4 hover:bg-[#e4e6f1] dark:hover:bg-[#2a2d2e] group">
                 <div class="flex items-center gap-2 overflow-hidden">
                     <span class="font-normal text-[11px] text-[#616161] dark:text-[#cccccc] uppercase whitespace-nowrap">
                        {move || t::source_control::title(locale.get())}
                     </span>
                     // Active Repo Indicator
                     <span class="text-[11px] font-bold text-[#3b3b3b] dark:text-[#white] truncate">
                        {active_repo_label}
                     </span>
                 </div>

                 <div class="flex gap-1 opacity-100 dark:text-[#cccccc] relative">
                    // Repo Switcher Button (Small Arrow)
                    <button
                        class="p-1 hover:bg-[#0000001a] dark:hover:bg-[#ffffff1a] rounded flex items-center justify-center transform transition-transform"
                        class:rotate-180=move || show_repo_switcher.get()
                        title="Switch Repository"
                        on:click=move |e| {
                            e.stop_propagation();
                            show_repo_switcher.update(|v| *v = !*v);
                            show_menu.set(false); // Close other menu
                        }
                    >
                         <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7 10l5 5 5-5"/></svg>
                    </button>

                    // More Actions Button
                    <button
                        class="p-1 hover:bg-[#0000001a] dark:hover:bg-[#ffffff1a] rounded"
                        title="More Actions"
                        on:click=move |e| {
                            e.stop_propagation();
                            show_menu.update(|v| *v = !*v);
                            show_repo_switcher.set(false); // Close repo menu
                        }
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/></svg>
                    </button>

                    // Repo Switcher Dropdown
                    {move || if show_repo_switcher.get() {
                        view! {
                            <div
                                class="absolute right-0 top-full mt-1 w-48 bg-white dark:bg-[#252526] border border-[#e5e5e5] dark:border-[#454545] shadow-lg rounded z-50 text-[12px] py-1 max-h-64 overflow-y-auto"
                                on:click=move |e| e.stop_propagation()
                            >
                                <div class="px-3 py-1 text-[10px] uppercase text-[#616161] font-bold border-b border-[#e5e5e5] dark:border-[#454545] mb-1">
                                    Repositories
                                </div>
                                <For
                                    each=move || core.repo_list.get()
                                    key=|repo| repo.clone()
                                    children=move |repo_name| {
                                        let repo_name_c = repo_name.clone();
                                        let is_active = repo_name == active_repo_label.get();
                                        view! {
                                            <div
                                                class="px-3 py-1.5 hover:bg-[#e8e8e8] dark:hover:bg-[#37373d] cursor-pointer flex items-center justify-between"
                                                class:bg-blue-50=move || is_active
                                                class:dark:bg-blue-900=move || is_active
                                                on:click=move |_| {
                                                    core.set_active_repo.set(Some(PeerId(repo_name_c.clone())));
                                                    show_repo_switcher.set(false);
                                                }
                                            >
                                                <div class="flex items-center gap-2">
                                                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3 text-blue-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3"></ellipse><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"></path><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"></path></svg>
                                                    <span>{repo_name}</span>
                                                </div>
                                                {if is_active {
                                                    view! { <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3 text-blue-600" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg> }.into_any()
                                                } else {
                                                    view! {}.into_any()
                                                }}
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}

                    // Context Menu
                    {move || if show_menu.get() {
                        view! {
                            <div
                                class="absolute right-0 top-full mt-1 w-32 bg-white dark:bg-[#252526] border border-[#e5e5e5] dark:border-[#454545] shadow-lg rounded z-50 text-[12px] py-1"
                                on:click=move |e| e.stop_propagation()
                            >
                                <div
                                    class="px-3 py-1.5 hover:bg-[#e8e8e8] dark:hover:bg-[#37373d] cursor-pointer flex items-center justify-between"
                                    on:click=move |_| { show_changes.update(|v| *v = !*v); }
                                >
                                    <span>{move || t::source_control::changes(locale.get())}</span>
                                    {move || if show_changes.get() {
                                        view! { <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg> }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                </div>
                                <div
                                    class="px-3 py-1.5 hover:bg-[#e8e8e8] dark:hover:bg-[#37373d] cursor-pointer flex items-center justify-between"
                                    on:click=move |_| { show_graph.update(|v| *v = !*v); }
                                >
                                    <span>{move || t::source_control::graph(locale.get())}</span>
                                    {move || if show_graph.get() {
                                        view! { <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg> }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                 </div>
            </div>

            // Scrollable Content
            <div class="flex-1 overflow-y-auto">
                // Removed Repositories List

                // Changes Section (Changes + Commit)
                {move || if show_changes.get() {
                    view! {
                        <div class="border-t border-[#e5e5e5] dark:border-[#252526]">
                            <button
                                 class="w-full flex items-center px-1 py-0.5 hover:bg-[#e8e8e8] dark:hover:bg-[#2a2d2e] text-[11px] font-bold text-[#424242] dark:text-[#cccccc] uppercase group"
                                 on:click=move |_| expand_changes.update(|b| *b = !*b)
                            >
                                <span class=move || if expand_changes.get() { "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform" } else { "w-4 h-4 flex items-center justify-center transition-transform" }>
                                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>
                                </span>
                                <span class="flex-1 text-left">{move || t::source_control::changes(locale.get())}</span>
                                <div class="hidden group-hover:flex items-center gap-1">
                                    // Add action icons if needed, e.g. Discard All, Stage All (Placeholders)
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
                }}

                {move || if show_graph.get() {
                    view! { <History expanded=expand_history /> }.into_any()
                } else {
                    view! {}.into_any()
                }}

                <div class="h-8"></div>
            </div>
        </div>
    }
}
