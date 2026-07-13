use crate::api::WsService;
use crate::components::activity_bar::SidebarView;
use crate::components::command_palette::logic::create_filtered_commands_memo;
use crate::components::command_palette::types::Command;
use crate::components::main_layout::SidebarControl;
use crate::hooks::use_core::SourceControlContext;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::Locale;
use crate::runtime::domain::{PendingBranchSwitch, PendingRepoSwitch};
use crate::runtime::session_client::SessionClient;
use crate::runtime::source_control_client::diff_session::DiffSessionWire;
use deve_core::models::PeerId;
use deve_core::source_control::{
    ChangeEntry, CommitFileDiffSummary, CommitInfo, ConflictResolution,
};
use leptos::prelude::{
    Callback, GetUntracked, Memo, ReadSignal, RwSignal, Set, Signal, WriteSignal, provide_context,
    signal, use_context,
};

pub(super) fn provide_source_control_context() -> ReadSignal<Option<SourceControlNotice>> {
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
    let (commit_diff_result, set_commit_diff_result) = signal(Vec::<CommitFileDiffSummary>::new());
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

pub(super) fn provide_sidebar_control_context(
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

pub(super) fn command_contexts() -> (
    Option<WriteSignal<Option<SourceControlNotice>>>,
    Option<SidebarControl>,
) {
    (
        use_context::<SourceControlContext>().map(|source_control| source_control.set_notice),
        use_context::<SidebarControl>(),
    )
}

pub(super) fn assert_cli_notice_command(
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

pub(super) fn provide_session_client(ws: WsService) {
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

pub(super) fn create_commands(set_show: WriteSignal<bool>) -> Memo<Vec<Command>> {
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

pub(super) fn ngit_status_enabled_when(commands: Memo<Vec<Command>>) -> String {
    commands
        .get_untracked()
        .into_iter()
        .find(|command| command.id == "ngit_status")
        .expect("ngit status command")
        .enabled_when
}
