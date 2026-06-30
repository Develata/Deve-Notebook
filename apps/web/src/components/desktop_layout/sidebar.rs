//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::activity_bar::{ActivityBar, SidebarView};
use crate::components::focus_scope;
use crate::components::sidebar::Sidebar;
use crate::runtime::{document_client::DocumentClient, scope_client::ScopeClient};
use leptos::prelude::*;

#[component]
pub fn DesktopSidebar(
    sidebar_width: Signal<i32>,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
) -> impl IntoView {
    let document = expect_context::<DocumentClient>();
    let scope = expect_context::<ScopeClient>();
    let panel_ref = NodeRef::<leptos::html::Aside>::new();
    let (surface_hidden, set_surface_hidden) = signal(sidebar_width.get_untracked() == 0);

    Effect::new(move |_| {
        let hidden = sidebar_width.get() == 0;
        if !hidden {
            set_surface_hidden.set(false);
            return;
        }
        if let Some(panel) = panel_ref.get_untracked() {
            let root: &web_sys::Element = panel.as_ref();
            let _ = focus_scope::blur_active_element_inside(root);
        }
        set_surface_hidden.set(true);
    });

    view! {
        <aside
            node_ref=panel_ref
            data-deve-desktop-col="1-sidebar"
            data-deve-desktop-col-width=move || sidebar_width.get().to_string()
            aria-hidden=move || surface_hidden.get().to_string()
            class="min-w-0 bg-panel rounded-lg shadow-sm border border-default flex flex-col z-[var(--z-panels)] overflow-hidden"
            style=move || {
                if surface_hidden.get() {
                    "visibility: hidden; pointer-events: none;".to_string()
                } else {
                    String::new()
                }
            }
        >
            <ActivityBar
                active_view=active_view
                set_active_view=set_active_view
                pinned_views=pinned_views
                set_pinned_views=set_pinned_views
            />
            <div class="flex-1 overflow-hidden">
                <Sidebar
                    active_view=active_view
                    docs=document.docs
                    current_doc=document.current_doc
                    is_readonly=scope.is_spectator
                    on_select=document.on_doc_select
                    on_delete=document.on_doc_delete
                    on_search_open=Callback::new(|_| {})
                />
            </div>
        </aside>
    }
}
