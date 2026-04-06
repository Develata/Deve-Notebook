use crate::components::activity_bar::SidebarView;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub(super) fn LeftDrawerTabButton(
    locale: RwSignal<Locale>,
    view: SidebarView,
    active_view: ReadSignal<SidebarView>,
    select_view: Callback<SidebarView>,
    on_open_more: Callback<()>,
) -> impl IntoView {
    let label = Signal::derive(move || match view {
        SidebarView::Explorer => t::sidebar::explorer(locale.get()),
        SidebarView::Search => t::sidebar::search(locale.get()),
        SidebarView::SourceControl => t::sidebar::source_control(locale.get()),
        SidebarView::Extensions => t::sidebar::extensions(locale.get()),
    });

    view! {
        <button
            class=move || {
                let state = if active_view.get() == view {
                    "bg-accent-subtle border border-b-accent text-accent"
                } else {
                    "bg-panel border border-default text-secondary active:bg-hover"
                };
                format!(
                    "mobile-sidebar-tab {} h-11 min-w-12 px-3 rounded-md active:scale-95 transition-transform duration-150 ease-out {}",
                    sidebar_tab_class(view),
                    state
                )
            }
            on:click=move |_| {
                select_view.run(view);
                on_open_more.run(());
            }
            title=move || label.get().to_string()
            aria-label=move || label.get().to_string()
        >
            <div class="w-4 h-4 mx-auto">{view.icon_view("w-4 h-4")}</div>
        </button>
    }
}

fn sidebar_tab_class(view: SidebarView) -> &'static str {
    match view {
        SidebarView::Explorer => "mobile-tab-explorer",
        SidebarView::Search => "mobile-tab-search",
        SidebarView::SourceControl => "mobile-tab-source-control",
        SidebarView::Extensions => "mobile-tab-extensions",
    }
}
