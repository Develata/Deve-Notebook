//! plan_ref:
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!
//! Read-only Git repair review panel for Source Control notices.

use crate::components::sidebar::source_control::repair_review_copy::GitRepairReviewCopy;
use crate::i18n::{Locale, source_control as sc};
use leptos::prelude::*;

#[component]
pub(super) fn GitRepairReviewPanel(
    review: GitRepairReviewCopy,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let status_note_view = review
        .status_note
        .clone()
        .map(|note| {
            let attr = note.clone();
            view! {
                <p
                    class="mt-1 text-[11px] text-muted"
                    data-deve-git-repair-review-status=attr
                >
                    {note}
                </p>
            }
            .into_any()
        })
        .unwrap_or_else(|| view! {}.into_any());

    view! {
        <div
            class="mt-3 rounded-md border border-warning/30 bg-panel/80 p-2 text-xs"
            data-deve-git-repair-review="readonly"
        >
            <p class="font-medium text-primary">{review.title}</p>
            {status_note_view}
            <div class="mt-2 space-y-2">
                {review
                    .records
                    .into_iter()
                    .map(|record| {
                        let heading_attr = record.heading.clone();
                        let heading_text = record.heading.clone();
                        let retry_command_attr = record.retry_command.clone();
                        let retry_command_text = record.retry_command.clone();
                        view! {
                            <div class="rounded border border-default/70 bg-sidebar/50 p-2">
                                <p
                                    class="font-mono text-[11px] text-secondary"
                                    data-deve-git-repair-record=heading_attr
                                >
                                    {heading_text}
                                </p>
                                <div class="mt-1 space-y-1">
                                    {record
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
                                        {move || sc::git_repair_retry_command_label(locale.get())}
                                    </span>
                                    <code
                                        class="mt-1 block select-all rounded border border-default bg-panel px-2 py-1 font-mono text-[11px] text-primary"
                                        data-deve-git-repair-retry-command=retry_command_attr
                                    >
                                        {retry_command_text}
                                    </code>
                                </div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
            <p
                class="mt-2 border-l border-warning/30 pl-2 text-[11px] text-muted"
                data-deve-git-repair-manual-only="true"
            >
                {review.authority_note}
            </p>
        </div>
    }
}
