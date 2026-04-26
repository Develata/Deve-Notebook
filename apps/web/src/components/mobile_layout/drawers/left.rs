// apps/web/src/components/mobile_layout/drawers/left.rs

use crate::components::activity_bar::SidebarView;
use crate::components::sidebar::Sidebar;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

use super::drawer_class;

#[path = "left_header.rs"]
mod header;
#[path = "left_tabs.rs"]
mod tabs;

use header::LeftDrawerHeader;
use tabs::LeftDrawerTabs;

#[component]
pub fn LeftDrawer(
    core: CoreState,
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

    let title = Signal::derive(move || match active_view.get() {
        SidebarView::Explorer => t::sidebar::explorer(locale.get()).to_string(),
        SidebarView::Search => t::sidebar::search(locale.get()).to_string(),
        SidebarView::SourceControl => t::sidebar::source_control(locale.get()).to_string(),
        SidebarView::Extensions => t::sidebar::extensions(locale.get()).to_string(),
    });

    view! {
        <div class=move || drawer_class("left", open.get())>
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
                            docs=core.docs
                            current_doc=core.current_doc
                            is_readonly=core.is_spectator
                            on_select=Callback::new(move |id| {
                                on_doc_select.run(id);
                                on_close.run(())
                            })
                            on_delete=core.on_doc_delete
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}
