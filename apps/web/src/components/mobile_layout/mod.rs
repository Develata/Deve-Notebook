// apps/web/src/components/mobile_layout/mod.rs
//! # Mobile Layout

mod chat_sheet;
mod content;
mod drawers;
mod effects;
mod footer;
mod footer_details;
mod footer_playback;
mod footer_status;
mod footer_summary;
mod gesture;
mod header;
mod layout_backdrop;
mod layout_banner;
mod layout_frame;
mod layout_runtime;
mod outline_button;
mod toolbar;

use crate::components::activity_bar::SidebarView;
use crate::editor::ffi::getEditorContent;
use crate::hooks::use_core::CoreState;
use crate::i18n::Locale;
use effects::apply_body_scroll_lock;
use effects::apply_visual_viewport_offset;
use gesture::{build_touch_end, build_touch_start};
use layout_frame::MobileLayoutFrame;
use layout_runtime::{build_doc_select_callback, build_mobile_title, resolve_content_signal};
use leptos::prelude::*;

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

    let title = build_mobile_title(core.clone());
    let (content_signal, set_outline_content) = resolve_content_signal();
    let banner_toggle = core.clone();
    let banner_text = core.clone();

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

    let on_doc_select = build_doc_select_callback(core.on_doc_select, close_drawers);

    apply_body_scroll_lock(drawer_open);
    apply_visual_viewport_offset(set_keyboard_offset);

    Effect::new(move |_| {
        if show_outline.get() {
            set_outline_content.set(getEditorContent());
        }
    });

    view! {
        <MobileLayoutFrame
            core=core.clone()
            locale=locale
            title=title
            active_view=active_view
            set_active_view=set_active_view
            pinned_views=pinned_views
            set_pinned_views=set_pinned_views
            show_sidebar=show_sidebar
            set_show_sidebar=set_show_sidebar
            show_outline=show_outline
            set_show_outline=set_show_outline
            drawer_open=drawer_open
            keyboard_offset=keyboard_offset
            chat_expanded=chat_expanded
            set_chat_expanded=set_chat_expanded
            on_home=on_home
            on_open=on_open
            on_command=on_command
            on_doc_select=on_doc_select
            on_close_drawers=close_drawers
            banner_toggle=banner_toggle
            banner_text=banner_text
            content_signal=content_signal
            on_touch_start=on_touch_start
            on_touch_end=on_touch_end
            on_touch_cancel=Callback::new(move |_| set_swipe_target.set(None))
        />
    }
}
