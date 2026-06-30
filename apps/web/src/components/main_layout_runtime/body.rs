//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!   - 04_repository#repo-scope-runtime
//!
use crate::components::activity_bar::SidebarView;
use crate::components::desktop_layout::DesktopLayout;
use crate::components::mobile_layout::MobileLayout;
use crate::hooks::use_layout::LayoutHookReturn;
use leptos::prelude::*;

#[component]
pub fn MainLayoutBody(
    desktop_layout: LayoutHookReturn,
    is_mobile: ReadSignal<bool>,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    chat_visible: ReadSignal<bool>,
    sidebar_visible: ReadSignal<bool>,
    mobile_sidebar_visible: ReadSignal<bool>,
    set_mobile_sidebar_visible: WriteSignal<bool>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
    on_logout: Callback<()>,
) -> impl IntoView {
    view! {
        {move || if is_mobile.get() {
            view! {
                <MobileLayout
                    active_view=active_view
                    set_active_view=set_active_view
                    pinned_views=pinned_views
                    set_pinned_views=set_pinned_views
                    show_sidebar=mobile_sidebar_visible
                    set_show_sidebar=set_mobile_sidebar_visible
                    on_home=on_home
                    on_open=on_open
                    on_command=on_command
                    on_logout=on_logout
                />
            }
            .into_any()
        } else {
            view! {
                <DesktopLayout
                    layout=desktop_layout
                    active_view=active_view
                    set_active_view=set_active_view
                    pinned_views=pinned_views
                    set_pinned_views=set_pinned_views
                    sidebar_visible=sidebar_visible
                    on_home=on_home
                    on_open=on_open
                    on_command=on_command
                    on_logout=on_logout
                    chat_visible=chat_visible
                />
            }
            .into_any()
        }}

        {move || if !is_mobile.get() {
            view! { <crate::components::bottom_bar::BottomBar /> }
                .into_any()
        } else {
            view! {}.into_any()
        }}
    }
}
