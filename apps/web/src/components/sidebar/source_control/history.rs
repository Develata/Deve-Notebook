use crate::components::sidebar::source_control::history_timeline::HistoryTimeline;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn History(expanded: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let read_block = core.read_block;
    let history_loading = Signal::derive(move || core.commit_history_request_id.get().is_some());
    let read_blocked = Signal::derive(move || read_block.get().is_some());
    let selected_commit = RwSignal::new(Option::<String>::None);

    Effect::new(move |_| {
        if !expanded.get() || core.current_repo_id.get().is_none() || read_block.get().is_some() {
            return;
        }
        core.on_get_history.run(20);
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
                <div class="pb-2">
                    {move || {
                        if history_loading.get() {
                            view! {
                                <div class="px-6 pt-2 text-[12px] text-muted">
                                    {match locale.get() {
                                        Locale::En => "Loading history...",
                                        Locale::Zh => "正在加载历史记录...",
                                    }}
                                </div>
                            }.into_any()
                        } else if core.commit_history.get().is_empty() && core.notice.get().is_none() {
                            view! {
                                <div class="px-6 pt-2 text-[12px] text-muted">
                                    {match locale.get() {
                                        Locale::En => "No commit history yet on this branch.",
                                        Locale::Zh => "这个分支上还没有提交历史。",
                                    }}
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <HistoryTimeline
                                    locale
                                    read_blocked
                                    selected_commit
                                    commit_history=core.commit_history
                                    commit_diff_request_id=core.commit_diff_request_id
                                    commit_diff_result=core.commit_diff_result
                                    notice=core.notice
                                    set_commit_diff_result=core.set_commit_diff_result
                                    on_get_commit_diff=core.on_get_commit_diff
                                />
                            }.into_any()
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}
