//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!
use crate::components::command_palette::types::Command;
use crate::hooks::use_core::{SourceControlContext, source_control_notice::SourceControlNotice};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(super) fn establish_branch_command(locale: Locale, set_show: WriteSignal<bool>) -> Command {
    let source_control = use_context::<SourceControlContext>();
    Command::unavailable(
        "establish_branch",
        (t::command_palette::establish_branch)(locale),
        (t::command_palette::establish_branch_unavailable_reason)(locale),
        Callback::new(move |_| {
            if let Some(source_control) = source_control.clone() {
                source_control
                    .set_notice
                    .set(Some(SourceControlNotice::establish_branch_unavailable()));
            }
            set_show.set(false);
        }),
    )
    .with_group((t::command_palette::group_peer)(locale))
    .with_enabled_when((t::command_palette::establish_branch_unavailable_reason)(
        locale,
    ))
}

#[cfg(test)]
mod tests {
    use super::establish_branch_command;
    use crate::hooks::use_core::diff_session::DiffSessionWire;
    use crate::hooks::use_core::source_control_notice::{
        SourceControlNotice, is_establish_branch_unavailable_notice,
    };
    use crate::hooks::use_core::write_gate::RepoWriteBlock;
    use crate::hooks::use_core::{PendingBranchTarget, SourceControlContext};
    use crate::i18n::Locale;
    use deve_core::models::PeerId;
    use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, ConflictResolution};
    use leptos::prelude::{
        Callable, Callback, GetUntracked, ReadSignal, Set, Signal, provide_context, signal,
    };
    use leptos::reactive::owner::Owner;

    fn provide_source_control_context() -> ReadSignal<Option<SourceControlNotice>> {
        let (staged_changes, _) = signal(Vec::<ChangeEntry>::new());
        let (unstaged_changes, _) = signal(Vec::<ChangeEntry>::new());
        let (confirmed_changes, _) = signal(Vec::<ChangeEntry>::new());
        let (commit_history, _) = signal(Vec::<CommitInfo>::new());
        let (commit_history_request_id, _) = signal(None::<String>);
        let (commit_diff_request_id, set_commit_diff_request_id) = signal(None::<String>);
        let (notice, set_notice) = signal(None::<SourceControlNotice>);
        let (current_repo_id, _) = signal(Some("default".to_string()));
        let (current_scope_nonce, _) = signal(1u64);
        let (active_branch, _) = signal(None::<PeerId>);
        let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
        let (pending_repo_switch, _) = signal(None::<String>);
        let (git_bridge_mode, _) = signal("unknown".to_string());
        let (diff_content, set_diff_content) = signal(None::<DiffSessionWire>);
        let (commit_diff_result, set_commit_diff_result) = signal(Vec::<CommitFileDiff>::new());
        let clear_notice = Callback::new(move |_| set_notice.set(None));

        provide_context(SourceControlContext {
            staged_changes,
            unstaged_changes,
            confirmed_changes,
            commit_history,
            commit_history_request_id,
            commit_diff_request_id,
            set_commit_diff_request_id,
            can_write: Signal::derive(|| true),
            write_block: Signal::derive(|| None::<RepoWriteBlock>),
            read_block: Signal::derive(|| None::<RepoWriteBlock>),
            git_bridge_mode,
            notice,
            set_notice,
            clear_notice,
            current_repo_id,
            current_scope_nonce,
            active_branch,
            pending_branch_switch,
            pending_repo_switch,
            on_get_changes: Callback::new(|_| {}),
            on_stage_file: Callback::new(|_: ChangeEntry| {}),
            on_stage_files: Callback::new(|_: Vec<ChangeEntry>| {}),
            on_unstage_file: Callback::new(|_: ChangeEntry| {}),
            on_unstage_files: Callback::new(|_: Vec<ChangeEntry>| {}),
            on_discard_file: Callback::new(|_: ChangeEntry| {}),
            on_discard_pending: Callback::new(|_| {}),
            on_commit: Callback::new(|_: String| {}),
            on_get_history: Callback::new(|_: u32| {}),
            diff_content,
            set_diff_content,
            on_get_doc_diff: Callback::new(|_: ChangeEntry| {}),
            commit_diff_result,
            set_commit_diff_result,
            on_resolve_conflict: Callback::new(|_: (ChangeEntry, ConflictResolution)| {}),
            on_get_commit_diff: Callback::new(|_: (Option<String>, String)| {}),
            on_commit_and_push: Callback::new(|_: String| {}),
        });

        notice
    }

    #[test]
    fn acc_cmd_004a_establish_branch_command_is_unavailable_notice_only() {
        // CMD-004A: unimplemented P2P branch creation is an unavailable notice.
        let owner = Owner::new();
        owner.with(|| {
            let notice = provide_source_control_context();
            let (show, set_show) = signal(true);
            let command = establish_branch_command(Locale::En, set_show);

            assert!(command.availability.is_unavailable());
            assert_eq!(command.group, "P2P / Branch");

            command.action.run(());

            assert!(!show.get_untracked());
            let notice = notice.get_untracked().expect("source control notice");
            assert!(is_establish_branch_unavailable_notice(&notice));
        });
    }
}
