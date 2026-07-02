//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 18_release#runtime-observability
//!
use crate::api::fetch_git_mirror_repair_review;
use crate::components::sidebar::source_control::error_notice_copy as copy;
use crate::components::sidebar::source_control::repair_review_copy::{
    self as repair_copy, GitRepairReviewFetchState,
};
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::source_control_notice::{
    is_git_repair_cli_notice, is_git_status_cli_notice,
};
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[cfg(test)]
pub(crate) fn error_notice_visible(
    read_blocked: bool,
    notice: Option<&SourceControlNotice>,
    suppress_git_status_notice: bool,
) -> bool {
    visible_error_notice(read_blocked, notice, suppress_git_status_notice).is_some()
}

pub(crate) fn visible_error_notice<'a>(
    read_blocked: bool,
    notice: Option<&'a SourceControlNotice>,
    suppress_git_status_notice: bool,
) -> Option<&'a SourceControlNotice> {
    if read_blocked {
        return None;
    }

    notice.filter(|notice| !(suppress_git_status_notice && is_git_status_cli_notice(notice)))
}

fn visible_notice_snapshot(
    notice: ReadSignal<Option<SourceControlNotice>>,
    block: Signal<Option<RepoWriteBlock>>,
    suppress_git_status_notice: bool,
) -> Option<SourceControlNotice> {
    let notice_value = notice.get();
    visible_error_notice(
        block.get().is_some(),
        notice_value.as_ref(),
        suppress_git_status_notice,
    )
    .cloned()
}

#[component]
pub fn ErrorNotice(
    notice: ReadSignal<Option<SourceControlNotice>>,
    block: Signal<Option<RepoWriteBlock>>,
    current_repo_id: ReadSignal<Option<String>>,
    current_scope_nonce: ReadSignal<u64>,
    clear_notice: Callback<()>,
    suppress_git_status_notice: bool,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let repair_review = RwSignal::new(GitRepairReviewFetchState::Idle);

    Effect::new(move |_| {
        let notice_value = notice.get();
        let should_fetch =
            block.get().is_none() && notice_value.as_ref().is_some_and(is_git_repair_cli_notice);
        let repo_id = current_repo_id.get();

        if !should_fetch {
            repair_review.set(GitRepairReviewFetchState::Idle);
            return;
        }

        repair_review.set(GitRepairReviewFetchState::Loading);
        let scope_nonce = current_scope_nonce.get_untracked();
        spawn_local(async move {
            let fetched = fetch_git_mirror_repair_review(repo_id.clone(), scope_nonce).await;
            let still_current = current_repo_id.get_untracked() == repo_id
                && current_scope_nonce.get_untracked() == scope_nonce
                && notice
                    .get_untracked()
                    .as_ref()
                    .is_some_and(is_git_repair_cli_notice);
            if still_current {
                repair_review.set(match fetched {
                    Ok(review) => GitRepairReviewFetchState::Loaded(review),
                    Err(_) => GitRepairReviewFetchState::Failed,
                });
            }
        });
    });

    view! {
        <Show when=move || {
            visible_notice_snapshot(notice, block, suppress_git_status_notice).is_some()
        }>
            <div class="px-4 py-3 text-sm border-b border-default bg-warning/5">
                <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                        <p class="text-primary font-medium">
                            {move || {
                                visible_notice_snapshot(notice, block, suppress_git_status_notice)
                                    .map(|current| copy::title(locale.get(), &current))
                                    .unwrap_or_default()
                            }}
                        </p>
                        <p class="mt-1 text-xs text-muted">
                            {move || {
                                visible_notice_snapshot(notice, block, suppress_git_status_notice)
                                    .map(|current| copy::hint(locale.get(), &current))
                                    .unwrap_or_default()
                            }}
                        </p>
                        <div class="mt-2 space-y-1 text-xs text-muted">
                            {move || {
                                visible_notice_snapshot(notice, block, suppress_git_status_notice)
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
                                visible_notice_snapshot(notice, block, suppress_git_status_notice)
                                    .filter(is_git_repair_cli_notice)
                                    .map(|_| repair_copy::git_repair_review(locale.get(), &repair_review.get()))
                                    .map(|review| {
                                        let status_note = review.status_note.clone();
                                        let status_note_view = status_note
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
                                                                            {crate::i18n::source_control::git_repair_retry_command_label(locale.get())}
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
                                        .into_any()
                                    })
                                    .unwrap_or_else(|| view! {}.into_any())
                            }}
                        </div>
                    </div>
                    <button
                        type="button"
                        class="text-xs text-secondary hover:text-primary"
                        title=move || t::source_control::dismiss_notice(locale.get())
                        aria-label=move || t::source_control::dismiss_notice(locale.get())
                        on:click=move |_| clear_notice.run(())
                    >
                        {"×"}
                    </button>
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::{error_notice_visible, visible_error_notice};
    use crate::hooks::use_core::source_control_notice::SourceControlNotice;

    #[test]
    fn mobile_source_control_read_gate_hides_git_status_notice_only() {
        let status_notice = SourceControlNotice::git_status_cli_only();
        let mirror_notice = SourceControlNotice::git_mirror_cli_only();
        let export_notice = SourceControlNotice::git_export_cli_only();
        let import_notice = SourceControlNotice::git_import_cli_only();
        let push_notice = SourceControlNotice::git_push_cli_only();
        let repair_notice = SourceControlNotice::git_repair_cli_only();
        let source_control_notice = SourceControlNotice::establish_branch_unavailable();

        assert!(!error_notice_visible(false, Some(&status_notice), true));
        assert!(error_notice_visible(false, Some(&status_notice), false));
        for notice in [
            &mirror_notice,
            &export_notice,
            &import_notice,
            &push_notice,
            &repair_notice,
        ] {
            assert!(error_notice_visible(false, Some(notice), true));
        }
        assert!(error_notice_visible(
            false,
            Some(&source_control_notice),
            true,
        ));
        assert!(!error_notice_visible(
            true,
            Some(&source_control_notice),
            true
        ));
        assert!(!error_notice_visible(false, None, true));
    }

    #[test]
    fn mobile_source_control_read_gate_filters_rendered_git_status_notice() {
        let status_notice = SourceControlNotice::git_status_cli_only();
        let repair_notice = SourceControlNotice::git_repair_cli_only();
        let source_control_notice = SourceControlNotice::establish_branch_unavailable();

        assert_eq!(
            visible_error_notice(false, Some(&status_notice), true),
            None
        );
        assert_eq!(
            visible_error_notice(false, Some(&repair_notice), true),
            Some(&repair_notice)
        );
        assert_eq!(
            visible_error_notice(false, Some(&source_control_notice), true),
            Some(&source_control_notice)
        );
        assert_eq!(
            visible_error_notice(false, Some(&status_notice), false),
            Some(&status_notice)
        );
        assert_eq!(
            visible_error_notice(true, Some(&source_control_notice), true),
            None
        );
    }
}
