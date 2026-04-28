//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 03_rendering#document-authority-bridge
//!
use crate::components::sidebar::source_control::history_compare_banner::HistoryCompareBanner;
use crate::components::sidebar::source_control::history_timeline::HistoryTimeline;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn HistoryBody(
    locale: RwSignal<Locale>,
    history_loading: Signal<bool>,
    read_blocked: Signal<bool>,
    selected_commit: RwSignal<Option<String>>,
    compare_base_commit_id: RwSignal<Option<String>>,
    clear_compare_base: Callback<()>,
    use_selected_as_base: Callback<()>,
) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();

    view! {
        <div class="pb-2">
            <HistoryCompareBanner
                locale
                compare_base_commit_id
                commit_history=core.commit_history
                selected_commit
                clear_compare_base
                use_selected_as_base
            />
            {move || {
                if history_loading.get() {
                    view! {
                        <div class="px-6 pt-2 text-[12px] text-muted">
                            {move || t::source_control::loading_history(locale.get())}
                        </div>
                    }
                    .into_any()
                } else if core.commit_history.get().is_empty() && core.notice.get().is_none() {
                    view! {
                        <div class="px-6 pt-2 text-[12px] text-muted">
                            {move || t::source_control::no_commit_history(locale.get())}
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <HistoryTimeline
                            locale
                            read_blocked
                            selected_commit
                            compare_base_commit_id
                            commit_history=core.commit_history
                            commit_diff_request_id=core.commit_diff_request_id
                            set_commit_diff_request_id=core.set_commit_diff_request_id
                            commit_diff_result=core.commit_diff_result
                            notice=core.notice
                            set_notice=core.set_notice
                            set_commit_diff_result=core.set_commit_diff_result
                            on_get_commit_diff=core.on_get_commit_diff
                        />
                    }
                    .into_any()
                }
            }}
        </div>
    }
}
