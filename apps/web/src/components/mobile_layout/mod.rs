// apps/web/src/components/mobile_layout/mod.rs
//! # Mobile Layout

mod chat_sheet;
mod content;
mod drawers;
mod effects;
mod footer;
mod footer_playback;
mod footer_status;
mod gesture;
mod header;
mod outline_button;
mod toolbar;

use crate::components::activity_bar::SidebarView;
use crate::components::layout_context::EditorContentContext;
use crate::editor::ffi::getEditorContent;
use crate::hooks::use_core::CoreState;
use crate::i18n::Locale;
use chat_sheet::MobileChatSheet;
use content::MobileContent;
use drawers::MobileDrawers;
use effects::apply_body_scroll_lock;
use effects::apply_visual_viewport_offset;
use footer::MobileFooter;
use gesture::{build_touch_end, build_touch_start};
use header::MobileHeader;
use leptos::prelude::*;
use outline_button::OutlineToggleButton;
use toolbar::MobileAccessoryToolbar;

#[component]
pub fn MobileLayout(
    core: CoreState,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let (show_sidebar, set_show_sidebar) = signal(false);
    let (show_outline, set_show_outline) = signal(false);
    let drawer_open = Signal::derive(move || show_sidebar.get() || show_outline.get());
    let (swipe_start_x, set_swipe_start_x) = signal(0i32);
    let (swipe_target, set_swipe_target) = signal(None::<gesture::SwipeTarget>);
    let (keyboard_offset, set_keyboard_offset) = signal(0i32);
    let (chat_expanded, set_chat_expanded) = signal(false);

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
            class="flex flex-col flex-1 overflow-hidden bg-sidebar"
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
                <OutlineToggleButton
                    show_outline=show_outline
                    set_show_outline=set_show_outline
                    set_show_sidebar=set_show_sidebar
                    locale=locale
                />
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
                set_active_view=set_active_view
                pinned_views=pinned_views
                set_pinned_views=set_pinned_views
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
                    current_doc.get().is_some()
                        && diff_content.get().is_none()
                        && !drawer_open.get()
                        && keyboard_offset.get() > 0
                        && !chat_expanded.get()
                })
            />

            <MobileChatSheet
                keyboard_offset=keyboard_offset
                drawer_open=drawer_open
                diff_open=Signal::derive(move || diff_content.get().is_some())
                expanded=chat_expanded
                set_expanded=set_chat_expanded
            />

            <Show when=move || keyboard_offset.get() <= 0 && !chat_expanded.get()>
                <MobileFooter core=core.clone() />
            </Show>
        </div>
    }
}
