//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::components::icons::MoreHorizontal;
use crate::components::sidebar::source_control::context_menu::SectionMenu;
use crate::i18n::{Locale, t};
use leptos::ev::MouseEvent;
use leptos::prelude::*;

#[component]
pub fn SourceControlHeader(
    locale: RwSignal<Locale>,
    git_bridge_mode: ReadSignal<String>,
    show_menu: RwSignal<bool>,
    show_repos: RwSignal<bool>,
    show_changes: RwSignal<bool>,
    show_graph: RwSignal<bool>,
) -> impl IntoView {
    let toggle_menu = move |e: MouseEvent| {
        e.stop_propagation();
        show_menu.update(|v| *v = !*v);
    };

    view! {
        <div class="flex-none h-9 flex items-center justify-between px-4 hover:bg-hover group border-b border-transparent hover:border-default relative">
            <div class="flex items-center gap-2 overflow-hidden">
                <span class="font-normal text-[11px] text-secondary uppercase whitespace-nowrap">
                    {move || t::source_control::title(locale.get())}
                </span>
                <span
                    class="text-[10px] leading-none px-1.5 py-0.5 rounded border border-default text-muted whitespace-nowrap"
                    title=move || t::source_control::git_bridge_mode_title(locale.get())
                >
                    {move || {
                        t::source_control::git_bridge_mode_badge(
                            locale.get(),
                            &git_bridge_mode.get(),
                        )
                    }}
                </span>
            </div>

            <div
                class="flex gap-1 opacity-100 relative"
                on:click=move |e: MouseEvent| e.stop_propagation()
            >
                <button
                    type="button"
                    class="p-1 hover:bg-hover rounded focus:outline-none focus-visible:ring-1 focus-visible:ring-accent/40"
                    data-deve-sc-section-menu-trigger="true"
                    aria-haspopup="menu"
                    aria-controls="source-control-section-menu"
                    aria-expanded=move || show_menu.get().to_string()
                    aria-label=move || t::sidebar::more_actions(locale.get())
                    title=move || t::sidebar::more_actions(locale.get())
                    on:click=toggle_menu
                >
                    <MoreHorizontal class="w-3.5 h-3.5" />
                </button>

                <SectionMenu
                    show_menu
                    show_repos
                    show_changes
                    show_graph
                    locale
                />
            </div>
        </div>
    }
}
