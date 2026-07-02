//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::hooks::use_core::SourceControlContext;
use crate::hooks::use_core::navigation::PendingNavigation;
use leptos::prelude::*;

#[component]
pub fn MainLayoutOverlays(
    is_mobile: ReadSignal<bool>,
    show_search: ReadSignal<bool>,
    set_show_search: WriteSignal<bool>,
    search_mode: ReadSignal<String>,
    show_settings: ReadSignal<bool>,
    set_show_settings: WriteSignal<bool>,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    source_control_context: SourceControlContext,
    pending_navigation: ReadSignal<Option<PendingNavigation>>,
    set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
) -> impl IntoView {
    view! {
        <crate::components::search_box::UnifiedSearch
            show=show_search
            set_show=set_show_search
            mode_signal=Signal::derive(move || search_mode.get())
            ui_mode=Signal::derive(move || {
                if is_mobile.get() {
                    crate::components::search_box::SearchUiMode::Sheet
                } else {
                    crate::components::search_box::SearchUiMode::Overlay
                }
            })
            on_settings=on_settings
            on_open=on_open
            source_control_context=source_control_context
        />

        <crate::components::settings::SettingsModal
            show=show_settings
            set_show=set_show_settings
        />

        <crate::components::merge_modal_slot::MergeModalSlot />
        <crate::components::pending_navigation_modal::PendingNavigationModal
            pending=pending_navigation
            set_pending=set_pending_navigation
        />
    }
}
