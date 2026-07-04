//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::components::icons::MoreHorizontal;
use crate::components::sidebar::source_control::context_menu::SectionMenu;
use crate::i18n::{Locale, t};
use leptos::ev::MouseEvent;
use leptos::prelude::*;

use super::touch_target::{header_container_class, header_menu_trigger_class};

#[component]
pub fn SourceControlHeader(
    locale: RwSignal<Locale>,
    source_control_authority: ReadSignal<String>,
    show_menu: RwSignal<bool>,
    show_repos: RwSignal<bool>,
    show_changes: RwSignal<bool>,
    show_graph: RwSignal<bool>,
    show_history: RwSignal<bool>,
) -> impl IntoView {
    let toggle_menu = move |e: MouseEvent| {
        e.stop_propagation();
        show_menu.update(|v| *v = !*v);
    };

    view! {
        <div class=header_container_class()>
            <div class="flex items-center gap-2 overflow-hidden">
                <span class="font-normal text-[11px] text-secondary uppercase whitespace-nowrap">
                    {move || t::source_control::title(locale.get())}
                </span>
                <span
                    class="text-[10px] leading-none px-1.5 py-0.5 rounded border border-default text-muted whitespace-nowrap"
                    title=move || t::source_control::source_control_authority_title(locale.get())
                >
                    {move || {
                        t::source_control::source_control_authority_badge(
                            locale.get(),
                            &source_control_authority.get(),
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
                    class=header_menu_trigger_class()
                    data-deve-sc-section-menu-trigger="true"
                    data-deve-mobile-touch-target="source_control_header_menu"
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
                    show_history
                    locale
                />
            </div>
        </div>
    }
}
