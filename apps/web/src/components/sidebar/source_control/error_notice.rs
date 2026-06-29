//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 18_release#runtime-observability
//!
mod repair_review;

use crate::api::fetch_git_mirror_repair_review;
use crate::components::sidebar::source_control::error_notice_copy as copy;
use crate::components::sidebar::source_control::repair_review_copy::{
    self as repair_copy, GitRepairReviewFetchState,
};
use crate::hooks::use_core::source_control_notice::{
    SourceControlNotice, is_git_repair_cli_notice, is_local_command_notice,
};
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::Locale;
use leptos::prelude::*;
use leptos::task::spawn_local;

use self::repair_review::GitRepairReviewPanel;

fn should_show_notice(block: Option<RepoWriteBlock>, notice: Option<&SourceControlNotice>) -> bool {
    notice.is_some_and(|notice| block.is_none() || is_local_command_notice(notice))
}

fn repair_review_fetch_still_current(
    block: Option<RepoWriteBlock>,
    fetched_repo_id: Option<&str>,
    current_repo_id: Option<&str>,
    fetched_scope_nonce: u64,
    current_scope_nonce: u64,
    notice: Option<&SourceControlNotice>,
) -> bool {
    block.is_none()
        && fetched_repo_id == current_repo_id
        && fetched_scope_nonce == current_scope_nonce
        && notice.is_some_and(is_git_repair_cli_notice)
}

#[component]
pub fn ErrorNotice(
    notice: ReadSignal<Option<SourceControlNotice>>,
    block: Signal<Option<RepoWriteBlock>>,
    current_repo_id: ReadSignal<Option<String>>,
    current_scope_nonce: ReadSignal<u64>,
    clear_notice: Callback<()>,
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
            let current_repo = current_repo_id.get_untracked();
            let current_notice = notice.get_untracked();
            let still_current = repair_review_fetch_still_current(
                block.get_untracked(),
                repo_id.as_deref(),
                current_repo.as_deref(),
                scope_nonce,
                current_scope_nonce.get_untracked(),
                current_notice.as_ref(),
            );
            if still_current {
                repair_review.set(match fetched {
                    Ok(review) => GitRepairReviewFetchState::Loaded(review),
                    Err(_) => GitRepairReviewFetchState::Failed,
                });
            }
        });
    });

    view! {
        <Show when=move || should_show_notice(block.get(), notice.get().as_ref())>
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
                                    .filter(is_git_repair_cli_notice)
                                    .map(|_| repair_copy::git_repair_review(locale.get(), &repair_review.get()))
                                    .map(|review| {
                                        view! {
                                            <GitRepairReviewPanel review=review locale=locale/>
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
    use super::{repair_review_fetch_still_current, should_show_notice};
    use crate::hooks::use_core::source_control_notice::SourceControlNotice;
    use crate::hooks::use_core::write_gate::RepoWriteBlock;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn local_cli_notice_displays_even_when_read_gate_is_blocked() {
        let notice = SourceControlNotice::git_push_cli_only();

        assert!(should_show_notice(
            Some(RepoWriteBlock::HandshakingRepo),
            Some(&notice)
        ));
    }

    #[test]
    fn server_notice_still_respects_read_gate() {
        let notice = SourceControlNotice {
            code: ServerErrorCode::ScDocNotFound,
            detail: None,
        };

        assert!(!should_show_notice(
            Some(RepoWriteBlock::HandshakingRepo),
            Some(&notice)
        ));
        assert!(should_show_notice(None, Some(&notice)));
    }

    #[test]
    fn repair_review_fetch_result_is_discarded_when_scope_becomes_blocked_or_stale() {
        let repair_notice = SourceControlNotice::git_repair_cli_only();
        let server_notice = SourceControlNotice {
            code: ServerErrorCode::ScRepoContextInvalid,
            detail: None,
        };

        assert!(repair_review_fetch_still_current(
            None,
            Some("repo-a"),
            Some("repo-a"),
            7,
            7,
            Some(&repair_notice),
        ));
        assert!(!repair_review_fetch_still_current(
            Some(RepoWriteBlock::ScopeSwitching),
            Some("repo-a"),
            Some("repo-a"),
            7,
            7,
            Some(&repair_notice),
        ));
        assert!(!repair_review_fetch_still_current(
            None,
            Some("repo-a"),
            Some("repo-b"),
            7,
            7,
            Some(&repair_notice),
        ));
        assert!(!repair_review_fetch_still_current(
            None,
            Some("repo-a"),
            Some("repo-a"),
            7,
            8,
            Some(&repair_notice),
        ));
        assert!(!repair_review_fetch_still_current(
            None,
            Some("repo-a"),
            Some("repo-a"),
            7,
            7,
            Some(&server_notice),
        ));
    }
}
