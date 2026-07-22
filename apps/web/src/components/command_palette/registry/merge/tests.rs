use super::merge_peer_commands;
use crate::components::command_palette::logic::create_filtered_commands_memo;
use crate::components::command_palette::types::{Command, CommandAvailability};
use crate::hooks::use_core::{BranchContext, SyncMergeContext};
use crate::i18n::Locale;
use crate::runtime::domain::{PendingOpsPreview, SyncModeState};
use deve_core::models::PeerId;
use leptos::prelude::*;
use leptos::reactive::owner::Owner;
use std::sync::{Arc, Mutex};

#[test]
fn merge_peer_commands_use_explicit_peer_source_on_local_branch() {
    let owner = Owner::new();
    owner.with(|| {
        let (show, set_show) = signal(true);
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let commands = tracked_merge_peer_commands(
            Locale::En,
            set_show,
            branch_context(None, vec!["peer-a", "peer-b"]),
            sync_context(calls.clone()),
        );

        assert_eq!(commands.len(), 2);
        let command = commands
            .iter()
            .find(|command| command.id == "merge_peer_peer-a")
            .expect("peer-a merge command");
        assert_eq!(command.title, "P2P: Merge Peer: peer-a");
        command.action.run(());

        assert_eq!(calls.lock().unwrap().as_slice(), ["peer-a"]);
        assert!(!show.get_untracked());
    });
}

#[test]
fn merge_peer_commands_are_absent_on_remote_branch() {
    let owner = Owner::new();
    owner.with(|| {
        let (_, set_show) = signal(true);
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let commands = tracked_merge_peer_commands(
            Locale::En,
            set_show,
            branch_context(Some(PeerId::new("peer-a")), vec!["peer-a"]),
            sync_context(calls),
        );

        assert!(commands.is_empty());
    });
}

#[test]
fn command_palette_merge_peer_commands_follow_branch_signal() {
    let owner = Owner::new();
    owner.with(|| {
        let (_, set_show) = signal(true);
        let (query, _) = signal(String::new());
        let (active_branch, set_active_branch) = signal(None::<PeerId>);
        let branch = branch_context_from_signals(active_branch, set_active_branch, vec!["peer-a"]);
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        provide_context(branch);
        provide_context(sync_context(calls));

        let commands = create_filtered_commands_memo(
            query.into(),
            RwSignal::new(Locale::En),
            Callback::new(|_| {}),
            Callback::new(|_| {}),
            set_show,
            None,
        );

        assert!(
            commands
                .get_untracked()
                .iter()
                .any(|command| command.id == "merge_peer_peer-a")
        );

        set_active_branch.set(Some(PeerId::new("peer-a")));

        assert!(
            commands
                .get_untracked()
                .iter()
                .all(|command| !command.id.starts_with("merge_peer"))
        );
    });
}

#[test]
fn merge_peer_command_reports_no_source_on_empty_local_branch() {
    let owner = Owner::new();
    owner.with(|| {
        let (_, set_show) = signal(true);
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let commands = tracked_merge_peer_commands(
            Locale::En,
            set_show,
            branch_context(None, Vec::new()),
            sync_context(calls),
        );

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, "merge_peer");
        assert!(matches!(
            commands[0].availability,
            CommandAvailability::Unavailable { .. }
        ));
    });
}

fn tracked_merge_peer_commands(
    locale: Locale,
    set_show: WriteSignal<bool>,
    branch: BranchContext,
    sync: SyncMergeContext,
) -> Vec<Command> {
    Memo::new(move |_| {
        merge_peer_commands(locale, set_show, Some(branch.clone()), Some(sync.clone()))
    })
    .get_untracked()
}

fn branch_context(active: Option<PeerId>, shadows: Vec<&str>) -> BranchContext {
    let (active_branch, _) = signal(active);
    branch_context_from_signals(active_branch, signal(None::<PeerId>).1, shadows)
}

fn branch_context_from_signals(
    active_branch: ReadSignal<Option<PeerId>>,
    set_active_branch: WriteSignal<Option<PeerId>>,
    shadows: Vec<&str>,
) -> BranchContext {
    let (current_repo, set_current_repo) = signal(Some("default".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(Some("repo-1".to_string()));
    let (shadow_repos, _) = signal(shadows.into_iter().map(str::to_string).collect::<Vec<_>>());
    let (repo_list, _) = signal(vec!["default".to_string()]);
    let (repo_entries, _) = signal(Vec::new());
    let (removal_preview, _) = signal(None);
    BranchContext {
        active_branch,
        set_active_branch,
        on_switch_branch: Callback::new(|_: Option<String>| {}),
        current_repo,
        set_current_repo,
        current_repo_id,
        set_current_repo_id,
        on_switch_repo: Callback::new(|_| {}),
        on_create_repo: Callback::new(|_: String| {}),
        on_rename_repo: Callback::new(|_| {}),
        on_remove_repo: Callback::new(|_| {}),
        removal_preview,
        on_confirm_remove_repo: Callback::new(|_| {}),
        on_cancel_remove_repo: Callback::new(|_| {}),
        shadow_repos,
        on_list_shadows: Callback::new(|_| {}),
        repo_list,
        repo_entries,
    }
}

fn sync_context(calls: Arc<Mutex<Vec<String>>>) -> SyncMergeContext {
    let (sync_mode, _) = signal(SyncModeState::Auto);
    let (pending_ops_count, _) = signal(0u32);
    let (pending_ops_previews, _) = signal(Vec::<PendingOpsPreview>::new());
    SyncMergeContext {
        sync_mode,
        pending_ops_count,
        pending_ops_previews,
        on_get_sync_mode: Callback::new(|_| {}),
        on_set_sync_mode: Callback::new(|_: String| {}),
        on_get_pending_ops: Callback::new(|_| {}),
        on_confirm_merge: Callback::new(|_| {}),
        on_discard_pending: Callback::new(|_| {}),
        on_merge_peer: Callback::new(move |peer_id: String| {
            calls.lock().unwrap().push(peer_id);
        }),
    }
}
