use crate::components::activity_bar::SidebarView;
use crate::components::desktop_layout::DesktopLayout;
use crate::components::mobile_layout::MobileLayout;
use crate::hooks::use_core::CoreState;
use crate::hooks::use_layout::LayoutHookReturn;
use leptos::prelude::*;

#[component]
pub fn MainLayoutBody(
    core: CoreState,
    desktop_layout: LayoutHookReturn,
    is_mobile: ReadSignal<bool>,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    chat_visible: ReadSignal<bool>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
) -> impl IntoView {
    let core_for_layout = core.clone();
    let bottom_bar_core = core.clone();

    view! {
        {move || if is_mobile.get() {
            view! {
                <MobileLayout
                    core=core_for_layout.clone()
                    active_view=active_view
                    set_active_view=set_active_view
                    pinned_views=pinned_views
                    set_pinned_views=set_pinned_views
                    on_home=on_home
                    on_open=on_open
                    on_command=on_command
                />
            }
            .into_any()
        } else {
            view! {
                <DesktopLayout
                    core=core_for_layout.clone()
                    layout=desktop_layout
                    active_view=active_view
                    set_active_view=set_active_view
                    pinned_views=pinned_views
                    set_pinned_views=set_pinned_views
                    on_home=on_home
                    on_open=on_open
                    on_command=on_command
                    chat_visible=chat_visible
                />
            }
            .into_any()
        }}

        {move || if !is_mobile.get() {
            view! { <crate::components::bottom_bar::BottomBar core=bottom_bar_core.clone() /> }
                .into_any()
        } else {
            view! {}.into_any()
        }}
    }
}
