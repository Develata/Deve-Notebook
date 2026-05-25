// apps/web/src/components/mobile_layout/outline_button.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 10_rendering#document-authority-bridge
//!
//! # Outline Toggle Button (Mobile)
//!
//! Floating button to toggle the document outline panel.

use crate::components::icons::PanelLeft;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(super) fn outline_toggle_button_class(show_outline: bool) -> &'static str {
    if show_outline {
        "fixed z-[var(--z-floating)] h-11 w-11 p-1.5 rounded-md active:bg-accent-subtle transition-all duration-200 ease-out flex items-center justify-center"
    } else {
        "fixed z-[var(--z-floating)] h-11 w-11 p-1.5 rounded-md active:bg-hover transition-all duration-200 ease-out flex items-center justify-center"
    }
}

#[component]
pub fn OutlineToggleButton(
    show_outline: ReadSignal<bool>,
    set_show_outline: WriteSignal<bool>,
    set_show_sidebar: WriteSignal<bool>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    view! {
        <button
            data-deve-mobile-touch-target="outline_toggle"
            class=move || outline_toggle_button_class(show_outline.get())
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
                <PanelLeft class="w-5 h-5"/>
            </span>
        </button>
    }
}

#[cfg(test)]
mod tests {
    use super::outline_toggle_button_class;

    #[test]
    fn mobile_touch_targets_outline_toggle_is_at_least_44px() {
        for show_outline in [false, true] {
            let class = outline_toggle_button_class(show_outline);
            assert!(class.contains("h-11"));
            assert!(class.contains("w-11"));
        }
    }
}
