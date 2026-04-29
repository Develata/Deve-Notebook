//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 15_release#runtime-observability
//!
use crate::components::sidebar::source_control::error_notice_copy as copy;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::Locale;
use leptos::prelude::*;

#[component]
pub fn ErrorNotice(
    notice: ReadSignal<Option<SourceControlNotice>>,
    block: Signal<Option<RepoWriteBlock>>,
    clear_notice: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <Show when=move || block.get().is_none() && notice.get().is_some()>
            <div class="px-4 py-3 text-sm border-b border-default bg-warning/5">
                <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                        <p class="text-primary font-medium">
                            {move || {
                                notice
                                    .get()
                                    .map(|current| copy::title(locale.get(), &current))
                                    .unwrap_or_default()
                            }}
                        </p>
                        <p class="mt-1 text-xs text-muted">
                            {move || {
                                notice
                                    .get()
                                    .map(|current| copy::hint(locale.get(), &current))
                                    .unwrap_or_default()
                            }}
                        </p>
                        <div class="mt-2 space-y-1 text-xs text-muted">
                            {move || {
                                notice
                                    .get()
                                    .map(|current| {
                                        copy::details(locale.get(), &current)
                                            .into_iter()
                                            .map(|detail| {
                                                view! {
                                                    <p class="pl-3 border-l border-warning/30">
                                                        {detail}
                                                    </p>
                                                }
                                            })
                                            .collect_view()
                                            .into_any()
                                    })
                                    .unwrap_or_else(|| view! {}.into_any())
                            }}
                        </div>
                        <div>
                            {move || {
                                notice
                                    .get()
                                    .and_then(|current| copy::git_repair_review(locale.get(), &current))
                                    .map(|review| {
                                        let retry_command_attr = review.retry_command.clone();
                                        let retry_command_text = review.retry_command.clone();
                                        view! {
                                            <div
                                                class="mt-3 rounded-md border border-warning/30 bg-panel/80 p-2 text-xs"
                                                data-deve-git-repair-review="readonly"
                                            >
                                                <p class="font-medium text-primary">{review.title}</p>
                                                <div class="mt-2 space-y-1">
                                                    {review
                                                        .rows
                                                        .into_iter()
                                                        .map(|row| {
                                                            view! {
                                                                <div class="grid grid-cols-[88px_minmax(0,1fr)] gap-2">
                                                                    <span class="text-muted">{row.label}</span>
                                                                    <span class="text-secondary">{row.value}</span>
                                                                </div>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                                <div class="mt-2">
                                                    <span class="block text-muted">
                                                        {crate::i18n::source_control::git_repair_retry_command_label(locale.get())}
                                                    </span>
                                                    <code
                                                        class="mt-1 block select-all rounded border border-default bg-sidebar px-2 py-1 font-mono text-[11px] text-primary"
                                                        data-deve-git-repair-retry-command=retry_command_attr
                                                    >
                                                        {retry_command_text}
                                                    </code>
                                                </div>
                                                <p
                                                    class="mt-2 border-l border-warning/30 pl-2 text-[11px] text-muted"
                                                    data-deve-git-repair-manual-only="true"
                                                >
                                                    {review.authority_note}
                                                </p>
                                            </div>
                                        }
                                        .into_any()
                                    })
                                    .unwrap_or_else(|| view! {}.into_any())
                            }}
                        </div>
                    </div>
                    <button
                        class="text-xs text-secondary hover:text-primary"
                        on:click=move |_| clear_notice.run(())
                    >
                        {"×"}
                    </button>
                </div>
            </div>
        </Show>
    }
}
