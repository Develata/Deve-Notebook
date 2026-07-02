// apps/web/src/components/mobile_layout/drawers/left.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 04_repository#tree-projection-contract
//!

use crate::components::activity_bar::SidebarView;
use crate::components::mobile_layout::source_control_notice::clear_mobile_source_control_notice_for_drawer;
use crate::components::sidebar::Sidebar;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use crate::runtime::{document_client::DocumentClient, scope_client::ScopeClient};
use leptos::prelude::*;

use super::{drawer_aria_hidden, drawer_class};

mod header;
mod tabs;

use header::LeftDrawerHeader;
use tabs::LeftDrawerTabs;

pub(super) fn left_drawer_content_marker(open: bool) -> Option<&'static str> {
    open.then_some("visible")
}

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
    let source_control = use_context::<SourceControlContext>();

    let title = Signal::derive(move || match active_view.get() {
        SidebarView::Explorer => t::sidebar::explorer(locale.get()).to_string(),
        SidebarView::Search => t::sidebar::search(locale.get()).to_string(),
        SidebarView::SourceControl => t::sidebar::source_control(locale.get()).to_string(),
        SidebarView::Extensions => t::sidebar::extensions(locale.get()).to_string(),
    });

    Effect::new(move |_| {
        clear_mobile_source_control_notice_for_drawer(
            open.get(),
            active_view.get(),
            source_control.as_ref(),
        );
    });

    view! {
        <div
            data-deve-mobile-drawer="left"
            data-deve-mobile-drawer-open=move || open.get().to_string()
            aria-hidden=move || drawer_aria_hidden(open.get())
            class:pointer-events-none=move || !open.get()
            class=move || drawer_class("left", open.get())
        >
            <Show when=move || open.get()>
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

                    <div
                        class="flex-1 overflow-hidden px-2 pb-3"
                        style="padding-bottom: env(safe-area-inset-bottom);"
                        data-deve-mobile-drawer-content=move || left_drawer_content_marker(open.get())
                    >
                        <div class="h-full overflow-y-auto">
                            <Sidebar
                                active_view=active_view
                                docs=document.docs
                                current_doc=document.current_doc
                                is_readonly=scope.is_spectator
                                suppress_source_control_git_status_notice=true
                                on_select=Callback::new(move |id| {
                                    on_doc_select.run(id);
                                    on_close.run(())
                                })
                                on_delete=document.on_doc_delete
                                on_search_open=Callback::new(move |_| on_close.run(()))
                            />
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::left_drawer_content_marker;

    #[test]
    fn closed_mobile_left_drawer_does_not_render_panel_content() {
        assert_eq!(left_drawer_content_marker(false), None);
        assert_eq!(left_drawer_content_marker(true), Some("visible"));
    }
}
