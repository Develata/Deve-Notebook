// apps/web/src/components/mobile_layout/mod.rs
//! # Mobile Layout

mod content;
mod drawers;
mod effects;
mod footer;
mod gesture;
mod header;
mod toolbar;

use crate::components::activity_bar::SidebarView;
use crate::components::layout_context::EditorContentContext;
use crate::editor::ffi::getEditorContent;
use crate::hooks::use_core::CoreState;
use content::MobileContent;
use drawers::MobileDrawers;
use effects::apply_body_scroll_lock;
use effects::apply_visual_viewport_offset;
use footer::MobileFooter;
use gesture::{build_touch_end, build_touch_start};
use header::MobileHeader;
use leptos::prelude::*;
use toolbar::MobileAccessoryToolbar;

#[component]
pub fn MobileLayout(
    core: CoreState,
    active_view: ReadSignal<SidebarView>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
) -> impl IntoView {
    let (show_sidebar, set_show_sidebar) = signal(false);
    let (show_outline, set_show_outline) = signal(false);
    let drawer_open = Signal::derive(move || show_sidebar.get() || show_outline.get());
    let (swipe_start_x, set_swipe_start_x) = signal(0i32);
    let (swipe_target, set_swipe_target) = signal(None::<gesture::SwipeTarget>);
    let (keyboard_offset, set_keyboard_offset) = signal(0i32);

    let title = Memo::new(move |_| {
        let current = core.current_doc.get();
        if let Some(id) = current {
            let docs = core.docs.get();
            if let Some((_, path)) = docs.iter().find(|(doc_id, _)| *doc_id == id) {
                return path.clone();
            }
        }
        "Deve-Note".to_string()
    });

    let content_ctx = use_context::<EditorContentContext>();
    let (outline_content, set_outline_content) = signal(String::new());
    let content_signal = match content_ctx {
        Some(ctx) => Some(ctx.content),
        None => Some(outline_content),
    };
    let current_doc = core.current_doc;
    let diff_content = core.diff_content;

    let close_drawers = Callback::new(move |_| {
        set_show_sidebar.set(false);
        set_show_outline.set(false);
    });

    let on_touch_start = build_touch_start(
        show_sidebar,
        show_outline,
        set_swipe_start_x,
        set_swipe_target,
    );
    let on_touch_end = build_touch_end(
        swipe_target,
        swipe_start_x,
        set_show_sidebar,
        set_show_outline,
        close_drawers,
        set_swipe_target,
    );

    let on_doc_select = {
        let on_select = core.on_doc_select;
        let close = close_drawers.clone();
        Callback::new(move |id| {
            on_select.run(id);
            close.run(());
        })
    };

    apply_body_scroll_lock(drawer_open);
    apply_visual_viewport_offset(set_keyboard_offset);

    Effect::new(move |_| {
        if show_outline.get() {
            set_outline_content.set(getEditorContent());
        }
    });

    view! {
        <div
            class="flex flex-col flex-1 overflow-hidden bg-gray-50"
            style="touch-action: pan-y;"
            on:touchstart=move |ev| on_touch_start.run(ev)
            on:touchend=move |ev| on_touch_end.run(ev)
            on:touchcancel=move |_| set_swipe_target.set(None)
        >
            <MobileHeader
                title=title
                on_menu=Callback::new(move |_| {
                    set_show_outline.set(false);
                    set_show_sidebar.set(true);
                })
                on_home=on_home
                on_open=on_open
                on_command=on_command
            />

            <MobileContent core=core.clone() drawer_open=drawer_open />

            <Show when=move || current_doc.get().is_some() && diff_content.get().is_none() && !show_sidebar.get()>
                <button
                    class=move || {
                        if show_outline.get() {
                            "fixed z-[60] h-11 w-11 p-1.5 rounded-md active:bg-blue-100 transition-all duration-200 ease-out flex items-center justify-center"
                        } else {
                            "fixed z-[60] h-11 w-11 p-1.5 rounded-md active:bg-gray-100 transition-all duration-200 ease-out flex items-center justify-center"
                        }
                    }
                    style=move || {
                        if show_outline.get() {
                            "top: calc(env(safe-area-inset-top) + 54px); right: calc(min(78vw, 320px) - 8px);"
                        } else {
                            "top: calc(env(safe-area-inset-top) + 54px); right: 10px;"
                        }
                    }
                    title="Toggle Outline"
                    aria-label="Toggle Outline"
                    on:click=move |_| {
                        set_show_sidebar.set(false);
                        set_show_outline.update(|v| *v = !*v);
                    }
                >
                    <span class=move || if show_outline.get() {
                        "h-8 w-8 rounded-md border border-blue-200 bg-blue-50 text-blue-700 shadow-sm flex items-center justify-center"
                    } else {
                        "h-8 w-8 rounded-md border border-gray-200 bg-white text-gray-600 shadow-sm flex items-center justify-center"
                    }>
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5">
                            <path fill-rule="evenodd" d="M3 4.25A2.25 2.25 0 015.25 2h9.5A2.25 2.25 0 0117 4.25v11.5A2.25 2.25 0 0114.75 18h-9.5A2.25 2.25 0 013 15.75V4.25zM6 13a1 1 0 11-2 0 1 1 0 012 0zm0-5a1 1 0 11-2 0 1 1 0 012 0zm0-5a1 1 0 11-2 0 1 1 0 012 0zm3 10a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2zm7 5a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2z" clip-rule="evenodd" />
                        </svg>
                    </span>
                </button>
            </Show>

            {move || if drawer_open.get() {
                view! {
                    <div
                        class="fixed inset-0 bg-black/40 z-40 transition-opacity duration-200 ease-out"
                        on:click=move |_| close_drawers.run(())
                    ></div>
                }
                .into_any()
            } else {
                view! {}.into_any()
            }}

            <MobileDrawers
                core=core.clone()
                active_view=active_view
                show_sidebar=show_sidebar
                show_outline=show_outline
                on_doc_select=on_doc_select
                on_close=close_drawers
                content_signal=content_signal
            />

            <MobileAccessoryToolbar
                keyboard_offset=keyboard_offset
                readonly=core.is_spectator
                visible=Signal::derive(move || {
                    current_doc.get().is_some() && !drawer_open.get() && keyboard_offset.get() > 0
                })
            />

            <Show when=move || keyboard_offset.get() <= 0>
                <MobileFooter core=core.clone() />
            </Show>
        </div>
    }
}
