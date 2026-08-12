//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use super::types::Command;
use crate::components::focus_scope;
use crate::components::icons::{ArrowRight, Search, Zap};
use crate::components::overlay_lifecycle;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::{KeyboardEvent, MouseEvent};

pub(super) struct CommandPaletteOverlay {
    pub show: Signal<bool>,
    pub set_show: WriteSignal<bool>,
    pub query: Signal<String>,
    pub set_query: WriteSignal<String>,
    pub locale: RwSignal<Locale>,
    pub filtered_commands: Memo<Vec<Command>>,
    pub selected_index: Signal<usize>,
    pub set_selected_index: WriteSignal<usize>,
    pub handle_keydown: Arc<dyn Fn(KeyboardEvent) + Send + Sync>,
    pub input_ref: NodeRef<leptos::html::Input>,
}

pub(super) fn render_overlay(overlay: CommandPaletteOverlay) -> impl IntoView {
    let panel_ref = NodeRef::<leptos::html::Div>::new();
    let CommandPaletteOverlay {
        show,
        set_show,
        query,
        set_query,
        locale,
        filtered_commands,
        selected_index,
        set_selected_index,
        handle_keydown,
        input_ref,
    } = overlay;

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
                    aria-label=move || command_palette_dialog_label(locale.get())
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
                        <button
                            type="button"
                            data-deve-command-palette-close="true"
                            class="flex h-11 w-11 shrink-0 items-center justify-center rounded-md text-2xl leading-none text-primary hover:bg-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                            title=move || t::common::close(locale.get())
                            aria-label=move || t::common::close(locale.get())
                            on:click=move |event| {
                                overlay_lifecycle::close_from_control(event, set_show)
                            }
                        >
                            "×"
                        </button>
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
                                                let is_unavailable = cmd.availability.is_unavailable();
                                                let unavailable_attr = if is_unavailable {
                                                    cmd.id.clone()
                                                } else {
                                                    String::new()
                                                };
                                                let title = cmd.title.clone();
                                                let detail = cmd.detail_text();
                                                let metadata = cmd.metadata_text();
                                                let group = cmd.group.clone();
                                                let shortcut = cmd.shortcut.clone();
                                                view! {
                                                    <button
                                                        class=format!(
                                                            "w-full text-left px-4 py-3 rounded-lg flex items-center gap-3 group transition-colors {}",
                                                            if is_unavailable {
                                                                "text-muted opacity-75"
                                                            } else if is_sel {
                                                                "bg-accent-subtle text-accent"
                                                            } else {
                                                                "text-primary hover:bg-hover"
                                                            }
                                                        )
                                                        aria-disabled=if is_unavailable { "true" } else { "false" }
                                                        data-deve-command-unavailable=unavailable_attr
                                                        on:click=move |event| {
                                                            overlay_lifecycle::run_action_from_control(
                                                                event,
                                                                || cmd.action.run(()),
                                                            )
                                                        }
                                                        on:mousemove=move |_| set_selected_index.set(idx)
                                                    >
                                                        <div class=format!("flex-none {}", if is_sel && !is_unavailable { "text-accent" } else { "text-muted" })>
                                                            <Zap class="w-5 h-5"/>
                                                        </div>
                                                        <div class="flex-1 min-w-0">
                                                            <span class="block truncate font-medium">{title}</span>
                                                            <span class="mt-0.5 block truncate text-xs text-muted">
                                                                {detail}
                                                            </span>
                                                            <span class="mt-0.5 block truncate text-[11px] text-muted opacity-80">
                                                                {metadata}
                                                            </span>
                                                        </div>
                                                        <div class="hidden sm:flex shrink-0 flex-col items-end gap-1 text-[11px] text-muted">
                                                            <span class="max-w-28 truncate">{group}</span>
                                                            {shortcut
                                                                .map(|shortcut| {
                                                                    view! {
                                                                        <kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">
                                                                            {shortcut}
                                                                        </kbd>
                                                                    }
                                                                    .into_any()
                                                                })
                                                                .unwrap_or_else(|| view! {}.into_any())}
                                                        </div>
                                                        <Show when=move || is_sel && !is_unavailable>
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
                            <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Up/Down</kbd> " " {move || t::command_palette::keyboard_navigate_hint(locale.get())}</span>
                            <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Enter</kbd> " " {move || t::command_palette::keyboard_select_hint(locale.get())}</span>
                        </div>
                        <span><kbd class="font-sans bg-panel px-1.5 py-0.5 rounded border border-default">Esc</kbd> " " {move || t::command_palette::keyboard_close_hint(locale.get())}</span>
                    </div>
                </div>
            </div>
        </Show>
    }
}

fn command_palette_dialog_label(locale: Locale) -> &'static str {
    t::command_palette::dialog_label(locale)
}
