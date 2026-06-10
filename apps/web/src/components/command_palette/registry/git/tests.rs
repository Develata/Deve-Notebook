use super::{
    git_bridge_mode_from_node_role, git_export_command, git_import_command, git_mirror_command,
    git_push_command, git_repair_command, git_status_command,
};
use crate::components::command_palette::types::Command;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::hooks::use_core::source_control_notice::{
    SourceControlNotice, is_git_export_cli_notice, is_git_import_cli_notice,
    is_git_mirror_cli_notice, is_git_push_cli_notice, is_git_repair_cli_notice,
    is_git_status_cli_notice,
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

fn assert_cli_notice_command(
    command: Command,
    show: ReadSignal<bool>,
    notice: ReadSignal<Option<SourceControlNotice>>,
    is_expected_notice: fn(&SourceControlNotice) -> bool,
) {
    command.action.run(());

    assert!(!show.get_untracked());
    let notice = notice.get_untracked().expect("source control notice");
    assert!(is_expected_notice(&notice));
}

#[test]
fn git_import_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let command = git_import_command(Locale::En, set_show);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_import_cli_notice);
    });
}

#[test]
fn git_status_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let command = git_status_command(Locale::En, set_show);

        assert!(command.availability.is_unavailable());
        assert!(
            command
                .enabled_when
                .contains("source_control.git_bridge=unknown")
        );
        assert_cli_notice_command(command, show, notice, is_git_status_cli_notice);
    });
}

#[test]
fn git_mirror_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let command = git_mirror_command(Locale::En, set_show);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_mirror_cli_notice);
    });
}

#[test]
fn git_export_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let command = git_export_command(Locale::En, set_show);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_export_cli_notice);
    });
}

#[test]
fn git_push_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let command = git_push_command(Locale::En, set_show);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_push_cli_notice);
    });
}

#[test]
fn git_repair_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let command = git_repair_command(Locale::En, set_show);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_repair_cli_notice);
    });
}

#[test]
fn git_bridge_mode_is_extracted_from_node_role_summary() {
    assert_eq!(
        git_bridge_mode_from_node_role(
            "main (ws:3001) | v0.0.1 | standard | repos:healthy (0/1) | git:mirror",
        ),
        Some("mirror")
    );
    assert_eq!(
        git_bridge_mode_from_node_role(
            "main (ws:3001) | v0.0.1 | standard | repos:healthy (0/1) | git:off",
        ),
        Some("off")
    );
    assert_eq!(git_bridge_mode_from_node_role("main | no git mode"), None);
}
