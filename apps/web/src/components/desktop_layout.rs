// apps/web/src/components/desktop_layout.rs
//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! # Desktop Layout

use self::banner::DesktopSyncBanner;
use self::content::DesktopLayoutContent;
use self::handles::{DesktopInnerResizeHandle, DesktopOuterResizeHandle};
use self::sidebar::DesktopSidebar;
use crate::components::activity_bar::SidebarView;
use crate::components::desktop_chat_panel::DesktopChatPanel;
use crate::components::header::Header;
use crate::hooks::use_layout::{DESKTOP_DIVIDER_WIDTH, LayoutHookReturn};
use crate::runtime::session_client::SessionClient;
use leptos::prelude::*;

mod banner;
mod content;
mod editor_tabs;
mod handles;
mod sidebar;

#[component]
pub fn DesktopLayout(
    layout: LayoutHookReturn,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    sidebar_visible: ReadSignal<bool>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
    on_logout: Callback<()>,
    chat_visible: ReadSignal<bool>,
) -> impl IntoView {
    let session = expect_context::<SessionClient>();
    let layout_for_grid = layout;

    view! {
        <Header
            status_text=session.status_text
            on_home=on_home
            on_open=on_open
            on_command=on_command
            on_logout=on_logout
        />
        <DesktopSyncBanner sync_banner=session.sync_banner />
        <main
            class="flex-1 w-full grid overflow-hidden relative gap-0"
            style=move || {
                desktop_layout_main_style(
                    layout_for_grid.left_width.get(),
                    layout_for_grid.right_width.get(),
                    layout_for_grid.outer_gutter.get(),
                    sidebar_visible.get(),
                    chat_visible.get(),
                )
            }
        >
            <DesktopOuterResizeHandle
                side="left"
                outer_gutter=layout.outer_gutter
                on_resize=layout.start_outer_left_resize
            />
            <DesktopOuterResizeHandle
                side="right"
                outer_gutter=layout.outer_gutter
                on_resize=layout.start_outer_right_resize
            />

            {move || if sidebar_visible.get() {
                view! {
                    <DesktopSidebar
                        sidebar_width=layout.left_width
                        active_view=active_view
                        set_active_view=set_active_view
                        pinned_views=pinned_views
                        set_pinned_views=set_pinned_views
                    />

                    <DesktopInnerResizeHandle
                        marker="left-divider"
                        on_resize=layout.start_left_divider_resize
                    />
                }.into_any()
            } else {
                view! {}.into_any()
            }}

            <DesktopLayoutContent center_width=layout.center_width />

            <DesktopChatPanel
                chat_visible=chat_visible
                right_width=layout.right_width
                start_resize_right=layout.start_right_divider_resize
            />
        </main>
    }
}

pub(crate) fn desktop_layout_main_style(
    left_width: i32,
    right_width: i32,
    outer_gutter: i32,
    sidebar_visible: bool,
    chat_visible: bool,
) -> String {
    format!(
        "padding-left: {}px; padding-right: {}px; {}",
        outer_gutter.max(0),
        outer_gutter.max(0),
        desktop_layout_grid_template(left_width, right_width, sidebar_visible, chat_visible),
    )
}

pub(crate) fn desktop_layout_grid_template(
    left_width: i32,
    right_width: i32,
    sidebar_visible: bool,
    chat_visible: bool,
) -> String {
    let divider = format!("{}px", DESKTOP_DIVIDER_WIDTH);
    let columns = match (sidebar_visible, chat_visible) {
        (true, true) => format!(
            "minmax(0, {}px) {} minmax(0, 1fr) {} minmax(0, {}px)",
            left_width, divider, divider, right_width
        ),
        (true, false) => format!("minmax(0, {}px) {} minmax(0, 1fr)", left_width, divider),
        (false, true) => {
            format!("minmax(0, 1fr) {} minmax(0, {}px)", divider, right_width)
        }
        (false, false) => "minmax(0, 1fr)".to_string(),
    };

    format!("grid-template-columns: {columns};")
}

#[cfg(test)]
mod tests {
    use super::{desktop_layout_grid_template, desktop_layout_main_style};

    #[test]
    fn desktop_layout_resize_grid_keeps_two_divider_tracks_when_all_regions_visible() {
        let style = desktop_layout_grid_template(250, 350, true, true);
        assert_eq!(
            style,
            "grid-template-columns: minmax(0, 250px) 16px minmax(0, 1fr) 16px minmax(0, 350px);"
        );
    }

    #[test]
    fn desktop_layout_resize_grid_hides_chat_divider_when_chat_visibility_is_off() {
        let style = desktop_layout_grid_template(250, 350, true, false);
        assert_eq!(
            style,
            "grid-template-columns: minmax(0, 250px) 16px minmax(0, 1fr);"
        );
        assert!(!style.contains("350px"));
    }

    #[test]
    fn desktop_layout_resize_main_style_preserves_outer_gutter_padding() {
        let style = desktop_layout_main_style(250, 350, 24, true, true);
        assert!(style.contains("padding-left: 24px"));
        assert!(style.contains("padding-right: 24px"));
        assert!(style.contains("grid-template-columns:"));
    }
}
