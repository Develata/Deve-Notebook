use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

#[component]
pub fn MainLayoutOverlays(
    core: CoreState,
    is_mobile: ReadSignal<bool>,
    show_search: ReadSignal<bool>,
    set_show_search: WriteSignal<bool>,
    search_mode: ReadSignal<String>,
    show_settings: ReadSignal<bool>,
    set_show_settings: WriteSignal<bool>,
    on_settings: Callback<()>,
    on_open: Callback<()>,
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
        />

        <crate::components::settings::SettingsModal
            show=show_settings
            set_show=set_show_settings
        />

        <crate::components::merge_modal_slot::MergeModalSlot />
        <crate::components::pending_navigation_modal::PendingNavigationModal
            pending=core.pending_navigation
            set_pending=core.set_pending_navigation
        />
    }
}
