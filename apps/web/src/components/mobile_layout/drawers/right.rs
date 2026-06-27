// apps/web/src/components/mobile_layout/drawers/right.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 10_rendering#document-authority-bridge
//!

use crate::components::focus_scope;
use crate::components::outline::Outline;
use crate::editor::ffi::scroll_global;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

use super::{drawer_class, drawer_hidden_style};

pub(super) fn drawer_close_button_class() -> &'static str {
    "h-11 min-w-[44px] px-3 text-sm font-medium text-secondary rounded-md hover:bg-hover active:bg-active transition-colors duration-200 ease-out"
}

#[component]
pub fn RightDrawer(
    open: ReadSignal<bool>,
    on_close: Callback<()>,
    content_signal: Option<ReadSignal<String>>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let drawer_ref = NodeRef::<leptos::html::Div>::new();
    let (surface_hidden, set_surface_hidden) = signal(!open.get_untracked());

    Effect::new(move |_| {
        let hidden = !open.get();
        if !hidden {
            set_surface_hidden.set(false);
            return;
        }
        if let Some(drawer) = drawer_ref.get_untracked() {
            let root: &web_sys::Element = drawer.as_ref();
            let _ = focus_scope::blur_active_element_inside(root);
        }
        set_surface_hidden.set(true);
    });

    view! {
        <div
            node_ref=drawer_ref
            data-deve-mobile-drawer="right"
            data-deve-mobile-drawer-open=move || open.get().to_string()
            aria-hidden=move || surface_hidden.get().to_string()
            class=move || drawer_class("right", open.get())
            style=move || drawer_hidden_style(surface_hidden.get())
        >
            <div class="flex flex-col h-full">
                <div
                    class="h-12 px-3 flex items-center justify-between border-b border-default text-sm font-semibold"
                    style="padding-top: env(safe-area-inset-top);"
                >
                    <span class="text-primary flex items-center gap-1">
                        {move || t::sidebar::outline(locale.get())}
                    </span>
                    <button
                        data-deve-mobile-touch-target="drawer_close_buttons"
                        class=drawer_close_button_class()
                        title=move || t::sidebar::close_outline(locale.get())
                        aria-label=move || t::sidebar::close_outline(locale.get())
                        on:click=move |_| on_close.run(())
                    >
                        {move || t::sidebar::close_outline(locale.get())}
                    </button>
                </div>

                <div
                    class="flex-1 overflow-y-auto px-2 pb-3"
                    style="padding-bottom: env(safe-area-inset-bottom);"
                >
                    {move || {
                        if let Some(content) = content_signal {
                            view! {
                                <Outline
                                    content=content
                                    on_scroll=Callback::new(move |line| {
                                        let close = on_close.clone();
                                        request_animation_frame(move || {
                                            scroll_global(line);
                                            close.run(());
                                        });
                                    })
                                />
                            }
                            .into_any()
                        } else {
                            view! {
                                <div class="px-4 py-8 text-sm text-muted flex flex-col items-center gap-2">
                                    <div class="w-9 h-9 rounded-full bg-hover flex items-center justify-center text-muted">
                                        "∅"
                                    </div>
                                    <div class="text-secondary">{move || t::sidebar::outline_unavailable(locale.get())}</div>
                                    <div class="text-[11px] text-muted">{move || t::sidebar::no_headings_found(locale.get())}</div>
                                </div>
                            }
                            .into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::drawer_close_button_class;

    #[test]
    fn mobile_touch_targets_right_drawer_close_button_is_at_least_44px() {
        let class = drawer_close_button_class();
        assert!(class.contains("h-11"));
        assert!(class.contains("min-w-[44px]"));
    }
}
