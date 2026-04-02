use crate::components::sidebar::source_control::history_body::HistoryBody;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::CommitFileDiff;
use leptos::prelude::*;

fn reset_compare_state(
    selected_commit: RwSignal<Option<String>>,
    compare_base_commit_id: RwSignal<Option<String>>,
    set_commit_diff_request_id: WriteSignal<Option<String>>,
    set_commit_diff_result: WriteSignal<Vec<CommitFileDiff>>,
    set_notice: WriteSignal<
        Option<crate::hooks::use_core::source_control_notice::SourceControlNotice>,
    >,
) {
    compare_base_commit_id.set(None);
    selected_commit.set(None);
    set_commit_diff_request_id.set(None);
    set_commit_diff_result.set(Vec::new());
    set_notice.set(None);
}

#[component]
pub fn History(expanded: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let read_block = core.read_block;
    let history_loading = Signal::derive(move || core.commit_history_request_id.get().is_some());
    let read_blocked = Signal::derive(move || read_block.get().is_some());
    let selected_commit = RwSignal::new(Option::<String>::None);
    let compare_base_commit_id = RwSignal::new(Option::<String>::None);

    let clear_compare_base = Callback::new(move |_| {
        reset_compare_state(
            selected_commit,
            compare_base_commit_id,
            core.set_commit_diff_request_id,
            core.set_commit_diff_result,
            core.set_notice,
        );
    });

    let on_toggle_compare_base = Callback::new(move |commit_id: String| {
        let compare_base_commit_id = compare_base_commit_id;
        let set_commit_diff_request_id = core.set_commit_diff_request_id;
        let set_commit_diff_result = core.set_commit_diff_result;
        let set_notice = core.set_notice;
        selected_commit.set(None);
        set_commit_diff_request_id.set(None);
        set_commit_diff_result.set(Vec::new());
        set_notice.set(None);
        compare_base_commit_id.update(|base| {
            if base.as_deref() == Some(commit_id.as_str()) {
                *base = None;
            } else {
                *base = Some(commit_id);
            }
        });
    });

    let use_selected_as_base = Callback::new(move |_| {
        let Some(selected_id) = selected_commit.get_untracked() else {
            return;
        };
        on_toggle_compare_base.run(selected_id);
    });

    Effect::new(move |_| {
        if !expanded.get() || core.current_repo_id.get().is_none() || read_block.get().is_some() {
            return;
        }
        core.on_get_history.run(20);
    });

    Effect::new(move |_| {
        let has_file_diff = core.diff_content.get().is_some();
        if !expanded.get() || core.current_repo_id.get().is_none() || has_file_diff {
            reset_compare_state(
                selected_commit,
                compare_base_commit_id,
                core.set_commit_diff_request_id,
                core.set_commit_diff_result,
                core.set_notice,
            );
        }
    });

    view! {
        <div class="border-t border-default">
            <button
                class="w-full flex items-center px-1 py-0.5 hover:bg-hover text-[11px] font-bold text-primary uppercase"
                on:click=move |_| expanded.update(|is_open| *is_open = !*is_open)
            >
                <span class=move || if expanded.get() { "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform" } else { "w-4 h-4 flex items-center justify-center transition-transform" }>
                    <crate::components::icons::ChevronRight class="w-3 h-3" />
                </span>
                {move || t::source_control::graph(locale.get())}
            </button>
            <Show when=move || expanded.get()>
                <HistoryBody
                    locale
                    history_loading
                    read_blocked
                    selected_commit
                    compare_base_commit_id
                    clear_compare_base
                    use_selected_as_base
                />
            </Show>
        </div>
    }
}
