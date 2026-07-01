//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!
pub mod change_item;
pub mod change_item_actions;
pub mod change_item_conflict_actions;
pub mod change_item_content;
pub mod change_item_counterpart;
pub mod change_item_meta;
mod change_item_read_gate;
pub mod change_item_workspace_actions;
pub mod changes;
pub mod changes_panel;
pub mod commit;
pub mod commit_actions;
pub mod commit_ai;
mod commit_ai_runtime;
pub mod commit_controller;
pub mod commit_message_box;
pub mod confirmed_section;
pub mod context_menu;
pub mod error_notice;
pub mod error_notice_copy;
pub mod graph_panel;
pub mod header;
pub mod history;
pub mod history_body;
pub mod history_commit_details;
pub mod history_commit_item;
pub mod history_commit_state;
pub mod history_compare_banner;
pub mod history_compare_logic;
pub mod history_diff_row;
pub mod history_empty_state;
pub mod history_timeline;
pub mod repair_review_copy;
pub mod repositories;
pub mod staged_section_actions;
pub mod status_notice;
mod touch_target;

pub mod staged_section;
pub mod unstaged_section;
pub mod unstaged_section_actions;

use self::changes_panel::ChangesPanel;
use self::commit::Commit;
use self::error_notice::ErrorNotice;
use self::graph_panel::GraphPanel;
use self::header::SourceControlHeader;
use self::history::History;
use self::status_notice::StatusNotice;
use crate::hooks::use_core::{
    SourceControlContext, source_control_notice::is_git_status_cli_notice,
};
use leptos::prelude::*;

fn should_clear_suppressed_git_status_notice(
    suppress_git_status_cli_notice: bool,
    notice: Option<&crate::hooks::use_core::source_control_notice::SourceControlNotice>,
) -> bool {
    suppress_git_status_cli_notice && notice.is_some_and(is_git_status_cli_notice)
}

#[component]
pub fn SourceControlView(suppress_git_status_cli_notice: bool) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<crate::i18n::Locale>>().expect("locale context");

    let expand_repos = RwSignal::new(false);
    let expand_graph = RwSignal::new(false);
    let expand_history = RwSignal::new(false);

    let show_repos = RwSignal::new(true);
    let show_changes = RwSignal::new(true);
    let show_graph = RwSignal::new(false);

    let show_menu = RwSignal::new(false);

    Effect::new(move |_| {
        let notice = core.notice.get();
        if should_clear_suppressed_git_status_notice(
            suppress_git_status_cli_notice,
            notice.as_ref(),
        ) {
            core.clear_notice.run(());
        }
    });

    view! {
        <div
            class="h-full w-full bg-sidebar flex flex-col font-sans select-none overflow-hidden text-[13px] text-primary relative"
            on:click=move |_| show_menu.set(false)
        >
            <SourceControlHeader
                locale
                git_bridge_mode=core.git_bridge_mode
                show_menu
                show_repos
                show_changes
                show_graph
            />

            <div class="flex-1 overflow-y-auto">
                <crate::components::sidebar::source_control::repositories::RepositoriesSection
                    expanded=expand_repos
                    visible=show_repos
                />
                <Commit />
                <StatusNotice block=core.write_block />
                <ErrorNotice
                    notice=core.notice
                    block=core.read_block
                    current_repo_id=core.current_repo_id
                    current_scope_nonce=core.current_scope_nonce
                    clear_notice=core.clear_notice
                    suppress_git_status_cli_notice=suppress_git_status_cli_notice
                />
                <ChangesPanel visible=show_changes />
                <Show when=move || show_graph.get()>
                    <GraphPanel expanded=expand_graph />
                </Show>
                <History expanded=expand_history />

                <div class="h-8"></div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::should_clear_suppressed_git_status_notice;
    use crate::hooks::use_core::source_control_notice::SourceControlNotice;

    #[test]
    fn mobile_source_control_read_surface_clears_git_status_notice() {
        let status_notice = SourceControlNotice::git_status_cli_only();
        let push_notice = SourceControlNotice::git_push_cli_only();

        assert!(should_clear_suppressed_git_status_notice(
            true,
            Some(&status_notice),
        ));
        assert!(!should_clear_suppressed_git_status_notice(
            false,
            Some(&status_notice),
        ));
        assert!(!should_clear_suppressed_git_status_notice(
            true,
            Some(&push_notice),
        ));
        assert!(!should_clear_suppressed_git_status_notice(true, None));
    }
}
