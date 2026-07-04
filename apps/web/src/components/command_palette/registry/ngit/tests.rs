use super::{
    ngit_export_command, ngit_import_command, ngit_mirror_command, ngit_push_command,
    ngit_repair_command, ngit_status_command, show_ngit_status_notice_for_viewport,
};
use crate::api::{ConnectionStatus, WsService};
use crate::components::activity_bar::SidebarView;
use crate::components::command_palette::logic::create_filtered_commands_memo;
use crate::components::command_palette::types::Command;
use crate::components::main_layout::SidebarControl;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::hooks::use_core::source_control_notice::{
    SourceControlNotice, is_git_export_cli_notice, is_git_import_cli_notice,
    is_git_mirror_cli_notice, is_git_push_cli_notice, is_git_repair_cli_notice,
    is_git_status_cli_notice,
};
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::hooks::use_core::{PendingBranchSwitch, PendingRepoSwitch, SourceControlContext};
use crate::i18n::Locale;
use crate::runtime::session_client::SessionClient;
use deve_core::models::PeerId;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, ConflictResolution};
use leptos::prelude::{
    Callback, GetUntracked, Memo, ReadSignal, RwSignal, Set, Signal, WriteSignal, provide_context,
    signal, use_context,
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
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    let (source_control_authority, _) = signal("unknown".to_string());
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
        source_control_authority,
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

fn provide_sidebar_control_context(
    mobile: bool,
) -> (ReadSignal<bool>, ReadSignal<bool>, ReadSignal<SidebarView>) {
    let (is_mobile, _) = signal(mobile);
    let (sidebar_visible, set_sidebar_visible) = signal(false);
    let (mobile_visible, set_mobile_visible) = signal(false);
    let (active_view, set_active_view) = signal(SidebarView::Explorer);
    provide_context(SidebarControl {
        is_mobile,
        set_visible: set_sidebar_visible,
        set_mobile_visible,
        set_active_view,
    });
    (sidebar_visible, mobile_visible, active_view)
}

fn command_contexts() -> (
    Option<WriteSignal<Option<SourceControlNotice>>>,
    Option<SidebarControl>,
) {
    (
        use_context::<SourceControlContext>().map(|source_control| source_control.set_notice),
        use_context::<SidebarControl>(),
    )
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

fn provide_session_client(ws: WsService) {
    let (sync_banner, set_sync_banner) = signal(None::<String>);
    let (handshake_ready, _) = signal(true);
    let (handshake_scope_nonce, _) = signal(Some(1u64));
    provide_context(SessionClient {
        ws: ws.clone(),
        connection_status: ws.status,
        status_text: Signal::derive(|| "connected".to_string()),
        sync_banner: sync_banner.into(),
        set_sync_banner,
        handshake_ready,
        handshake_scope_nonce,
        on_retry_peer_registration: Callback::new(|_| {}),
    });
}

fn create_commands(set_show: WriteSignal<bool>) -> Memo<Vec<Command>> {
    let (query, _) = signal(String::new());
    create_filtered_commands_memo(
        query.into(),
        RwSignal::new(Locale::En),
        Callback::new(|_| {}),
        Callback::new(|_| {}),
        set_show,
        None,
    )
}

fn ngit_status_enabled_when(commands: Memo<Vec<Command>>) -> String {
    commands
        .get_untracked()
        .into_iter()
        .find(|command| command.id == "ngit_status")
        .expect("ngit status command")
        .enabled_when
}

#[test]
fn ngit_import_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_import_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_import_cli_notice);
    });
}

#[test]
fn ngit_status_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_status_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert!(
            command
                .enabled_when
                .contains("source_control.authority=unknown")
        );
        assert_cli_notice_command(command, show, notice, is_git_status_cli_notice);
    });
}

#[test]
fn ngit_status_command_routes_mobile_to_source_control_drawer() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (sidebar_visible, mobile_visible, active_view) = provide_sidebar_control_context(true);
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_status_command(Locale::En, set_show, source_control, sidebar_control);

        command.action.run(());

        assert!(!show.get_untracked());
        assert!(notice.get_untracked().is_none());
        assert!(!sidebar_visible.get_untracked());
        assert!(mobile_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
    });
}

#[test]
fn ngit_status_command_clears_stale_cli_notice_on_mobile() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let source_control = use_context::<SourceControlContext>().expect("source control context");
        source_control
            .set_notice
            .set(Some(SourceControlNotice::git_status_cli_only()));
        let (_, mobile_visible, active_view) = provide_sidebar_control_context(true);
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_status_command(Locale::En, set_show, source_control, sidebar_control);

        command.action.run(());

        assert!(!show.get_untracked());
        assert!(notice.get_untracked().is_none());
        assert!(mobile_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
    });
}

#[test]
fn ngit_status_command_does_not_write_notice_on_mobile_viewport_without_sidebar_context() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();

        assert!(sidebar_control.is_none());
        show_ngit_status_notice_for_viewport(
            source_control,
            sidebar_control,
            use_context::<SourceControlContext>(),
            set_show,
            true,
        );

        assert!(!show.get_untracked());
        assert!(notice.get_untracked().is_none());
    });
}

#[test]
fn ngit_status_command_keeps_desktop_cli_notice_without_sidebar_context() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();

        assert!(sidebar_control.is_none());
        show_ngit_status_notice_for_viewport(
            source_control,
            sidebar_control,
            use_context::<SourceControlContext>(),
            set_show,
            false,
        );

        assert!(!show.get_untracked());
        let notice = notice.get_untracked().expect("source control notice");
        assert!(is_git_status_cli_notice(&notice));
    });
}

#[test]
fn ngit_status_command_routes_notice_to_source_control_sidebar_on_desktop() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (sidebar_visible, mobile_visible, active_view) = provide_sidebar_control_context(false);
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_status_command(Locale::En, set_show, source_control, sidebar_control);

        assert_cli_notice_command(command, show, notice, is_git_status_cli_notice);
        assert!(sidebar_visible.get_untracked());
        assert!(!mobile_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
    });
}

#[test]
fn ngit_status_detail_text_exposes_ngit_authority() {
    let owner = Owner::new();
    owner.with(|| {
        provide_source_control_context();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        ws.complete_foreground_node_role_reprobe("main", "ngit", false, false);
        provide_session_client(ws);
        let (show, set_show) = signal(true);
        let command = create_commands(set_show)
            .get_untracked()
            .into_iter()
            .find(|command| command.id == "ngit_status")
            .expect("ngit status command");

        let reason = command.availability.reason().expect("unavailable reason");
        let detail = command.detail_text();

        assert!(detail.contains(reason));
        assert!(detail.contains("source_control.authority=ngit"));
        assert!(show.get_untracked());
    });
}

#[test]
fn ngit_mirror_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_mirror_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_mirror_cli_notice);
    });
}

#[test]
fn ngit_export_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_export_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_export_cli_notice);
    });
}

#[test]
fn ngit_push_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_push_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_push_cli_notice);
    });
}

#[test]
fn ngit_push_command_routes_notice_to_source_control_sidebar() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (sidebar_visible, mobile_visible, active_view) = provide_sidebar_control_context(false);
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_push_command(Locale::En, set_show, source_control, sidebar_control);

        assert_cli_notice_command(command, show, notice, is_git_push_cli_notice);
        assert!(sidebar_visible.get_untracked());
        assert!(!mobile_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
    });
}

#[test]
fn ngit_push_command_routes_notice_to_mobile_source_control_drawer() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (sidebar_visible, mobile_visible, active_view) = provide_sidebar_control_context(true);
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_push_command(Locale::En, set_show, source_control, sidebar_control);

        assert_cli_notice_command(command, show, notice, is_git_push_cli_notice);
        assert!(!sidebar_visible.get_untracked());
        assert!(mobile_visible.get_untracked());
        assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
    });
}

#[test]
fn ngit_repair_command_sets_cli_only_notice() {
    let owner = Owner::new();
    owner.with(|| {
        let notice = provide_source_control_context();
        let (show, set_show) = signal(true);
        let (source_control, sidebar_control) = command_contexts();
        let command = ngit_repair_command(Locale::En, set_show, source_control, sidebar_control);

        assert!(command.availability.is_unavailable());
        assert_cli_notice_command(command, show, notice, is_git_repair_cli_notice);
    });
}

#[test]
fn command_palette_source_control_authority_reads_session_signal() {
    let owner = Owner::new();
    owner.with(|| {
        provide_source_control_context();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        ws.complete_foreground_node_role_reprobe("main", "ngit", false, false);
        provide_session_client(ws);
        let (_, set_show) = signal(true);

        let enabled_when = ngit_status_enabled_when(create_commands(set_show));

        assert!(enabled_when.contains("source_control.authority=ngit"));
    });
}

#[test]
fn command_palette_source_control_authority_updates_after_node_role_probe() {
    let owner = Owner::new();
    owner.with(|| {
        provide_source_control_context();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        provide_session_client(ws.clone());
        let (_, set_show) = signal(true);
        let commands = create_commands(set_show);

        assert!(ngit_status_enabled_when(commands).contains("source_control.authority=unknown"));

        ws.complete_foreground_node_role_reprobe("main", "ngit", false, false);

        assert!(ngit_status_enabled_when(commands).contains("source_control.authority=ngit"));
    });
}
