// apps/web/src/components/mobile_layout/drawers/left.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 04_repository#tree-projection-contract
//!

use crate::components::activity_bar::SidebarView;
use crate::components::focus_scope;
use crate::components::sidebar::Sidebar;
use crate::i18n::{Locale, t};
use crate::runtime::{document_client::DocumentClient, scope_client::ScopeClient};
use leptos::prelude::*;

use super::{drawer_class, drawer_hidden_style};

mod header;
mod tabs;

use header::LeftDrawerHeader;
use tabs::LeftDrawerTabs;

#[component]
pub fn LeftDrawer(
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    open: ReadSignal<bool>,
    on_doc_select: Callback<deve_core::models::DocId>,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let search_control = expect_context::<crate::components::main_layout::SearchControl>();
    let document = expect_context::<DocumentClient>();
    let scope = expect_context::<ScopeClient>();
    let drawer_ref = NodeRef::<leptos::html::Div>::new();
    let (surface_hidden, set_surface_hidden) = signal(!open.get_untracked());

    let title = Signal::derive(move || match active_view.get() {
        SidebarView::Explorer => t::sidebar::explorer(locale.get()).to_string(),
        SidebarView::Search => t::sidebar::search(locale.get()).to_string(),
        SidebarView::SourceControl => t::sidebar::source_control(locale.get()).to_string(),
        SidebarView::Extensions => t::sidebar::extensions(locale.get()).to_string(),
    });

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
            data-deve-mobile-drawer="left"
            data-deve-mobile-drawer-open=move || open.get().to_string()
            aria-hidden=move || surface_hidden.get().to_string()
            class=move || drawer_class("left", open.get())
            style=move || drawer_hidden_style(surface_hidden.get())
        >
            <div class="flex flex-col h-full">
                <LeftDrawerHeader locale title on_close />

                <LeftDrawerTabs
                    locale
                    active_view
                    set_active_view
                    pinned_views
                    set_pinned_views
                    open
                    on_search=Callback::new(move |_| {
                        search_control.set_mode.set("?".to_string());
                        search_control.set_show.set(true);
                        on_close.run(());
                    })
                />

                <div class="flex-1 overflow-hidden px-2 pb-3" style="padding-bottom: env(safe-area-inset-bottom);">
                    <div class="h-full overflow-y-auto">
                        <Sidebar
                            active_view=active_view
                            docs=document.docs
                            current_doc=document.current_doc
                            is_readonly=scope.is_spectator
                            on_select=Callback::new(move |id| {
                                on_doc_select.run(id);
                                on_close.run(())
                            })
                            on_delete=document.on_doc_delete
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}
