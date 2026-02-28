// apps/web/src/components/mobile_layout/outline_button.rs
//! # Outline Toggle Button (Mobile)
//!
//! Floating button to toggle the document outline panel.

use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn OutlineToggleButton(
    show_outline: ReadSignal<bool>,
    set_show_outline: WriteSignal<bool>,
    set_show_sidebar: WriteSignal<bool>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    view! {
        <button
            class=move || {
                if show_outline.get() {
                    "fixed z-[60] h-11 w-11 p-1.5 rounded-md active:bg-accent-subtle transition-all duration-200 ease-out flex items-center justify-center"
                } else {
                    "fixed z-[60] h-11 w-11 p-1.5 rounded-md active:bg-hover transition-all duration-200 ease-out flex items-center justify-center"
                }
            }
            style=move || {
                if show_outline.get() {
                    "top: calc(env(safe-area-inset-top) + 54px); right: calc(min(78vw, 320px) - 8px);"
                } else {
                    "top: calc(env(safe-area-inset-top) + 54px); right: 10px;"
                }
            }
            title=move || t::header::toggle_outline(locale.get())
            aria-label=move || t::header::toggle_outline(locale.get())
            on:click=move |_| {
                set_show_sidebar.set(false);
                set_show_outline.update(|v| *v = !*v);
            }
        >
            <span class=move || if show_outline.get() {
                "h-8 w-8 rounded-md border border-accent bg-accent-subtle text-accent shadow-sm flex items-center justify-center"
            } else {
                "h-8 w-8 rounded-md border border-default bg-panel text-secondary shadow-sm flex items-center justify-center"
            }>
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5">
                    <path fill-rule="evenodd" d="M3 4.25A2.25 2.25 0 015.25 2h9.5A2.25 2.25 0 0117 4.25v11.5A2.25 2.25 0 0114.75 18h-9.5A2.25 2.25 0 013 15.75V4.25zM6 13a1 1 0 11-2 0 1 1 0 012 0zm0-5a1 1 0 11-2 0 1 1 0 012 0zm0-5a1 1 0 11-2 0 1 1 0 012 0zm3 10a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2zm7 5a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2z" clip-rule="evenodd" />
                </svg>
            </span>
        </button>
    }
}
