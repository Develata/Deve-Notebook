//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
use crate::components::activity_bar::SidebarView;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(super) fn sidebar_tab_marker(view: SidebarView) -> &'static str {
    match view {
        SidebarView::Explorer => "explorer",
        SidebarView::Search => "search",
        SidebarView::SourceControl => "source_control",
        SidebarView::ExternalChanges => "external_changes",
        SidebarView::RemoteImport => "remote_import",
        SidebarView::Extensions => "extensions",
    }
}

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
        SidebarView::ExternalChanges => t::sidebar::external_changes(locale.get()),
        SidebarView::RemoteImport => t::sidebar::remote_import(locale.get()),
        SidebarView::Extensions => t::sidebar::extensions(locale.get()),
    });

    view! {
        <button
            type="button"
            data-deve-mobile-sidebar-tab=sidebar_tab_marker(view)
            data-deve-mobile-sidebar-tab-active=move || (active_view.get() == view).to_string()
            class=move || {
                let state = if active_view.get() == view {
                    "bg-accent-subtle border border-b-accent text-accent"
                } else {
                    "bg-panel border border-default text-secondary active:bg-hover"
                };
                format!(
                    "mobile-sidebar-tab {} h-11 min-w-[48px] px-3 rounded-md active:scale-95 transition-transform duration-150 ease-out {}",
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

pub(super) fn sidebar_tab_class(view: SidebarView) -> &'static str {
    match view {
        SidebarView::Explorer => "mobile-tab-explorer",
        SidebarView::Search => "mobile-tab-search",
        SidebarView::SourceControl => "mobile-tab-source-control",
        SidebarView::ExternalChanges => "mobile-tab-external-changes",
        SidebarView::RemoteImport => "mobile-tab-remote-import",
        SidebarView::Extensions => "mobile-tab-extensions",
    }
}

#[cfg(test)]
mod tests {
    use super::{sidebar_tab_class, sidebar_tab_marker};
    use crate::components::activity_bar::SidebarView;

    #[test]
    fn mobile_sidebar_icon_tab_markers_cover_sidebar_entries() {
        assert_eq!(sidebar_tab_marker(SidebarView::Explorer), "explorer");
        assert_eq!(sidebar_tab_marker(SidebarView::Search), "search");
        assert_eq!(
            sidebar_tab_marker(SidebarView::SourceControl),
            "source_control"
        );
        assert_eq!(
            sidebar_tab_marker(SidebarView::ExternalChanges),
            "external_changes"
        );
        assert_eq!(
            sidebar_tab_marker(SidebarView::RemoteImport),
            "remote_import"
        );
        assert_eq!(sidebar_tab_marker(SidebarView::Extensions), "extensions");
    }

    #[test]
    fn mobile_sidebar_icon_tab_classes_cover_sidebar_entries() {
        assert_eq!(
            sidebar_tab_class(SidebarView::Explorer),
            "mobile-tab-explorer"
        );
        assert_eq!(sidebar_tab_class(SidebarView::Search), "mobile-tab-search");
        assert_eq!(
            sidebar_tab_class(SidebarView::SourceControl),
            "mobile-tab-source-control"
        );
        assert_eq!(
            sidebar_tab_class(SidebarView::ExternalChanges),
            "mobile-tab-external-changes"
        );
        assert_eq!(
            sidebar_tab_class(SidebarView::RemoteImport),
            "mobile-tab-remote-import"
        );
        assert_eq!(
            sidebar_tab_class(SidebarView::Extensions),
            "mobile-tab-extensions"
        );
    }

    #[test]
    fn mobile_external_changes_entry_visible() {
        assert_eq!(
            sidebar_tab_marker(SidebarView::ExternalChanges),
            "external_changes"
        );
        assert_eq!(
            sidebar_tab_class(SidebarView::ExternalChanges),
            "mobile-tab-external-changes"
        );
    }
}
