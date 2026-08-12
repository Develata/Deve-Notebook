// apps/web/src/components/mobile_layout/mod.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 11_ui_design/03_mobile#mobile-current-native-boundary
//!
//! # Mobile Layout

mod chat_sheet;
mod content;
mod drawers;
mod effects;
mod footer;
mod footer_details;
mod footer_playback;
mod footer_read;
mod footer_status;
mod footer_summary;
mod gesture;
#[cfg(test)]
mod gesture_test;
mod header;
mod layout_backdrop;
mod layout_banner;
mod layout_frame;
mod layout_runtime;
mod outline_button;
mod source_control_notice;
mod surface_runtime;
mod surface_switcher;
mod toolbar;

use crate::components::activity_bar::SidebarView;
use crate::components::main_layout::SearchControl;
use crate::components::ui_back::{UiBackCoordinator, UiBackLayer};
use crate::editor::ffi::try_get_editor_content;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::Locale;
use crate::runtime::document_client::DocumentClient;
use effects::apply_body_scroll_lock;
use effects::apply_visual_viewport_offset;
use gesture::{build_touch_end, build_touch_start, clear_swipe_session};
use layout_frame::MobileLayoutFrame;
use layout_runtime::{build_doc_select_callback, build_mobile_title, resolve_content_signal};
use leptos::prelude::*;
use source_control_notice::{
    clear_mobile_source_control_notice_for_active_view, clear_mobile_source_control_notice_for_view,
};

pub(crate) fn edge_swipe_left_drawer_view() -> SidebarView {
    SidebarView::Explorer
}

#[component]
pub fn MobileLayout(
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    show_sidebar: ReadSignal<bool>,
    set_show_sidebar: WriteSignal<bool>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
    on_logout: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let document = expect_context::<DocumentClient>();
    let (show_outline, set_show_outline) = signal(false);
    let drawer_open = Signal::derive(move || show_sidebar.get() || show_outline.get());
    let (swipe_session, set_swipe_session) = signal(None::<gesture::SwipeSession>);
    let (keyboard_offset, set_keyboard_offset) = signal(0i32);
    let (chat_expanded, set_chat_expanded) = signal(false);
    let source_control_context = use_context::<SourceControlContext>();
    let source_control_for_drawer = source_control_context.clone();
    let source_control_for_active_view = source_control_context.clone();
    let search_control = expect_context::<SearchControl>();
    let ui_back = expect_context::<UiBackCoordinator>();

    let title = build_mobile_title(document.clone());
    let (content_signal, set_outline_content) = resolve_content_signal();

    let close_drawers = Callback::new(move |_| {
        set_show_sidebar.set(false);
        set_show_outline.set(false);
    });
    let close_left_drawer = Callback::new(move |_| set_show_sidebar.set(false));
    let close_right_drawer = Callback::new(move |_| set_show_outline.set(false));
    let open_left_drawer = Callback::new(move |_| {
        search_control.set_show.set(false);
        clear_mobile_source_control_notice_for_view(
            active_view.get_untracked(),
            source_control_for_drawer.as_ref(),
        );
        set_show_outline.set(false);
        set_show_sidebar.set(true);
    });
    let open_left_drawer_from_gesture = open_left_drawer;
    let open_file_tree_drawer = Callback::new(move |_| {
        set_active_view.set(edge_swipe_left_drawer_view());
        open_left_drawer_from_gesture.run(());
    });
    let open_right_drawer = Callback::new(move |_| {
        set_show_outline.set(true);
        set_show_sidebar.set(false);
    });

    let on_touch_start = build_touch_start(show_sidebar, show_outline, set_swipe_session);
    let on_touch_end = build_touch_end(
        swipe_session,
        open_file_tree_drawer,
        open_right_drawer,
        close_left_drawer,
        close_right_drawer,
        set_swipe_session,
    );

    let on_doc_select = build_doc_select_callback(document.on_doc_select, close_drawers);

    ui_back.register(UiBackLayer::TransientSheet, move || {
        if chat_expanded.try_get_untracked() == Some(true) {
            set_chat_expanded.set(false);
            return true;
        }
        false
    });
    ui_back.register(UiBackLayer::Drawer, move || {
        let sidebar_open = show_sidebar.try_get_untracked() == Some(true);
        let outline_open = show_outline.try_get_untracked() == Some(true);
        if sidebar_open || outline_open {
            set_show_sidebar.set(false);
            set_show_outline.set(false);
            return true;
        }
        false
    });

    apply_body_scroll_lock(drawer_open);
    apply_visual_viewport_offset(set_keyboard_offset);

    Effect::new(move |_| {
        clear_mobile_source_control_notice_for_active_view(
            active_view.get(),
            source_control_for_active_view.as_ref(),
        );
    });

    Effect::new(move |_| {
        if show_outline.get()
            && let Some(content) = try_get_editor_content()
        {
            set_outline_content.set(content);
        }
    });

    view! {
        <MobileLayoutFrame
            locale=locale
            title=title
            active_view=active_view
            set_active_view=set_active_view
            pinned_views=pinned_views
            set_pinned_views=set_pinned_views
            show_sidebar=show_sidebar
            set_show_sidebar=set_show_sidebar
            on_open_left_drawer=open_left_drawer
            show_outline=show_outline
            set_show_outline=set_show_outline
            drawer_open=drawer_open
            keyboard_offset=keyboard_offset
            chat_expanded=chat_expanded
            set_chat_expanded=set_chat_expanded
            on_home=on_home
            on_open=on_open
            on_command=on_command
            on_logout=on_logout
            on_doc_select=on_doc_select
            on_close_drawers=close_drawers
            content_signal=content_signal
            on_touch_start=on_touch_start
            on_touch_end=on_touch_end
            on_touch_cancel=Callback::new(move |_| {
                set_swipe_session.update(clear_swipe_session)
            })
        />
    }
}
