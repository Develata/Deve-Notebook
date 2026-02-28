// apps/web/src/components/desktop_layout.rs
//! # Desktop Layout

use crate::components::activity_bar::SidebarView;
use crate::components::chat::ChatPanel;
use crate::components::diff_view::DiffView;
use crate::components::header::Header;
use crate::editor::Editor;
use crate::hooks::use_core::CoreState;
use crate::hooks::use_layout::LayoutHookReturn;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn DesktopLayout(
    core: CoreState,
    layout: LayoutHookReturn,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
    chat_visible: ReadSignal<bool>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (
        sidebar_width,
        right_width,
        outer_gutter,
        start_resize_left,
        start_resize_right,
        start_resize_outer_left,
        start_resize_outer_right,
        _stop_resize,
        _do_resize,
        _is_resizing,
    ) = layout;

    view! {
        <Header
            status_text=core.status_text
            on_home=on_home
            on_open=on_open
            on_command=on_command
        />
        <main
            class="flex-1 w-full flex overflow-hidden relative"
            style=move || {
                format!(
                    "padding-left: {}px; padding-right: {}px;",
                    outer_gutter.get(),
                    outer_gutter.get()
                )
            }
        >
            <div
                class="absolute top-0 h-full w-3 cursor-col-resize touch-none"
                style=move || format!("left: {}px; transform: translateX(-50%);", outer_gutter.get())
                on:pointerdown=move |ev| {
                    if let Some(target) = ev.target()
                        && let Ok(el) = target.dyn_into::<web_sys::Element>()
                    {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                    }
                    start_resize_outer_left.run(ev)
                }
            ></div>
            <div
                class="absolute top-0 h-full w-3 cursor-col-resize touch-none"
                style=move || format!("right: {}px; transform: translateX(50%);", outer_gutter.get())
                on:pointerdown=move |ev| {
                    if let Some(target) = ev.target()
                        && let Ok(el) = target.dyn_into::<web_sys::Element>()
                    {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                    }
                    start_resize_outer_right.run(ev)
                }
            ></div>

            <aside
                class="flex-none bg-panel rounded-lg shadow-sm border border-default flex flex-col z-20"
                style=move || format!("width: {}px", sidebar_width.get())
            >
                <crate::components::activity_bar::ActivityBar
                    active_view=active_view
                    set_active_view=set_active_view
                    pinned_views=pinned_views
                    set_pinned_views=set_pinned_views
                />
                <div class="flex-1 overflow-hidden">
                    <crate::components::sidebar::Sidebar
                        active_view=active_view
                        docs=core.docs
                        current_doc=core.current_doc
                        on_select=core.on_doc_select
                        on_delete=core.on_doc_delete
                    />
                </div>
            </aside>

            <div
                class="w-4 flex-none cursor-col-resize flex items-center justify-center hover:bg-accent-subtle group transition-colors touch-none"
                on:pointerdown=move |ev| {
                    if let Some(target) = ev.target()
                        && let Ok(el) = target.dyn_into::<web_sys::Element>()
                    {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                    }
                    start_resize_left.run(ev)
                }
            >
                <div class="w-[1px] h-8 bg-active group-hover:bg-accent transition-colors"></div>
            </div>

            <div class="flex-1 bg-panel shadow-sm border border-default rounded-lg overflow-hidden relative flex flex-col min-w-0">
                {move || {
                    if let Some(session) = core.diff_content.get() {
                        return view! {
                            <DiffView
                                repo_scope=core.current_repo.get().unwrap_or_default()
                                path=session.path
                                old_content=session.old_content
                                new_content=session.new_content
                                is_readonly=core.is_spectator.get()
                                on_close=Callback::new(move |_| core.set_diff_content.set(None))
                            />
                        }
                        .into_any();
                    }

                    match core.current_doc.get() {
                        Some(id) => view! { <Editor doc_id=id on_stats=core.on_stats /> }.into_any(),
                        None => view! {
                            <div class="flex items-center justify-center h-full text-muted">
                                {move || t::common::select_document_to_edit(locale.get())}
                            </div>
                        }
                        .into_any(),
                    }
                }}
            </div>

            {move || if chat_visible.get() {
                view! {
                    <div class="flex items-stretch ml-4">
                        <div
                            class="w-4 flex-none cursor-col-resize flex items-center justify-center hover:bg-accent-subtle group transition-colors touch-none"
                            on:pointerdown=move |ev| {
                                if let Some(target) = ev.target()
                                    && let Ok(el) = target.dyn_into::<web_sys::Element>()
                                {
                                    let _ = el.set_pointer_capture(ev.pointer_id());
                                }
                                start_resize_right.run(ev)
                            }
                        >
                            <div class="w-[1px] h-8 bg-active group-hover:bg-accent transition-colors"></div>
                        </div>
                        <div
                            class="flex-none bg-panel shadow-sm border border-default rounded-lg overflow-hidden flex flex-col"
                            style=move || format!("width: {}px", right_width.get())
                        >
                            <ChatPanel on_close=Callback::new(move |_| ()) />
                        </div>
                    </div>
                }
                .into_any()
            } else {
                view! {}.into_any()
            }}
        </main>
    }
}
