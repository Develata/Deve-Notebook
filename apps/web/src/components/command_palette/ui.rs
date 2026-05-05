//! plan_ref:
//!   - 12_commands#command-palette-shortcuts
//!   - 08_ui_design_01_web#web-layout-persistence
//!
use super::types::Command;
use crate::components::focus_scope;
use crate::components::icons::{ArrowRight, Search, Zap};
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::{KeyboardEvent, MouseEvent};

#[allow(clippy::too_many_arguments)]
pub(super) fn render_overlay(
    show: Signal<bool>,
    set_show: WriteSignal<bool>,
    query: Signal<String>,
    set_query: WriteSignal<String>,
    locale: RwSignal<Locale>,
    filtered_commands: Memo<Vec<Command>>,
    selected_index: Signal<usize>,
    set_selected_index: WriteSignal<usize>,
    handle_keydown: Arc<dyn Fn(KeyboardEvent) + Send + Sync>,
    input_ref: NodeRef<leptos::html::Input>,
) -> impl IntoView {
    let panel_ref = NodeRef::<leptos::html::Div>::new();

    view! {
        <Show when=move || show.get()>
            <div
                class="fixed inset-0 z-[var(--z-modal)] font-sans"
                on:click=move |_| set_show.set(false)
            >
                <div
                    node_ref=panel_ref
                    role="dialog"
                    aria-modal="true"
                    tabindex="-1"
                    class="absolute top-2 left-1/2 -translate-x-1/2 w-full max-w-xl bg-panel rounded-lg shadow-xl border border-default overflow-hidden flex flex-col max-h-[60vh] animate-in fade-in zoom-in-95 duration-100"
                    on:click=move |ev: MouseEvent| ev.stop_propagation()
                    on:keydown={
                        let handle_keydown = handle_keydown.clone();
                        move |ev| {
                            if focus_scope::handle_focus_trap_keydown(&ev, panel_ref) {
                                return;
                            }
                            handle_keydown(ev);
                        }
                    }
                >
                    <div class="p-3 border-b border-default flex items-center gap-3 bg-sidebar">
                        <Search class="w-4 h-4 text-muted"/>
                        <input
                            node_ref=input_ref
                            name="command-palette-query"
                            type="text"
                            class="flex-1 outline-none text-base bg-transparent text-primary placeholder:text-muted"
                            placeholder=move || t::command_palette::placeholder(locale.get())
                            prop:value=move || query.get()
                            on:input=move |ev| {
                                set_query.set(event_target_value(&ev));
                                set_selected_index.set(0);
                            }
                            autofocus
                        />
                    </div>

                    <div class="overflow-y-auto p-2">
                        {move || {
                            let cmds = filtered_commands.get();
                            if cmds.is_empty() {
                                view! {
                                    <div class="p-4 text-center text-muted text-sm">
                                        {move || t::command_palette::no_results(locale.get())}
                                    </div>
                                }
                                .into_any()
                            } else {
                                let count = cmds.len();
                                let current = selected_index.get();
                                let idx_sel = if current >= count { 0 } else { current };
                                view! {
                                    <div class="flex flex-col gap-1">
                                        <For
                                            each=move || cmds.clone().into_iter().enumerate()
                                            key=|(_, cmd)| cmd.id.clone()
                                            children=move |(idx, cmd)| {
                                                let is_sel = idx == idx_sel;
                                                view! {
                                                    <button
                                                        class=format!(
                                                            "w-full text-left px-4 py-3 rounded-lg flex items-center gap-3 group transition-colors {}",
                                                            if is_sel { "bg-accent-subtle text-accent" } else { "text-primary hover:bg-hover" }
                                                        )
                                                        on:click=move |_| cmd.action.run(())
                                                        on:mousemove=move |_| set_selected_index.set(idx)
                                                    >
                                                        <div class=format!("flex-none {}", if is_sel { "text-accent" } else { "text-muted" })>
                                                            <Zap class="w-5 h-5"/>
                                                        </div>
                                                        <div class="flex-1 truncate">
                                                            <span class="font-medium">{cmd.title.clone()}</span>
                                                        </div>
                                                        <Show when=move || is_sel>
                                                            <ArrowRight class="w-4 h-4 text-accent opacity-0 group-hover:opacity-100 transition-opacity"/>
                                                        </Show>
                                                    </button>
                                                }
                                            }
                                        />
                                    </div>
                                }
                                .into_any()
                            }
                        }}
                    </div>
                    <div class="bg-sidebar px-4 py-2 border-t border-default flex justify-between items-center text-xs text-muted">
                        <div class="flex gap-4">
                            <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Up/Down</kbd> to navigate</span>
                            <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Enter</kbd> to select</span>
                        </div>
                        <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Esc</kbd> to close</span>
                    </div>
                </div>
            </div>
        </Show>
    }
}
