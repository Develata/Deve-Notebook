//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 05_diff_logic#remote-projection-transport

use super::{RemoteProjectionPushIntent, remote_projection_commands};
use crate::api::{ConnectionStatus, WsService};
use crate::i18n::Locale;
use crate::runtime::domain::{
    PendingRepoSwitch, RepoRemoveRequest, RepoRenameRequest, RepoSwitchRequest,
};
use crate::runtime::scope_client::ScopeClient;
use crate::runtime::session_client::SessionClient;
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::{ClientMessage, RemoteProjectionProvider, RepoListEntry};
use leptos::prelude::*;

#[test]
fn remote_projection_commands_expose_only_stable_push_ids() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let (_, set_show) = signal(true);
        let commands = remote_projection_commands(Locale::En, set_show);
        let ids = commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            ["remote_projection.webdav.push", "remote_projection.s3.push"]
        );
        assert!(
            commands
                .iter()
                .all(|command| command.availability.is_unavailable())
        );
        assert!(commands.iter().all(|command| {
            let detail = command.detail_text();
            !detail.contains("pull") && !detail.contains("External Changes")
        }));
    });
}

#[test]
fn handshake_ready_command_carries_exact_backend_scope_without_frontend_transport_io() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let repo_id = RepoId::new_v4();
        let branch = Some(PeerId::new("peer-a"));
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        provide_context(test_session_client(ws.clone(), true, Some(9)));
        provide_context(test_scope_client(
            Some(repo_id.to_string()),
            branch.clone(),
            9,
        ));
        let (show_palette, set_show_palette) = signal(true);
        let commands = Memo::new(move |_| remote_projection_commands(Locale::En, set_show_palette))
            .get_untracked();
        let push = commands
            .iter()
            .find(|command| command.id == "remote_projection.webdav.push")
            .expect("WebDAV push command");

        assert!(!push.availability.is_unavailable());
        push.action.run(());

        assert!(!show_palette.get_untracked());
        let sent = ws.drain_sent_for_test();
        assert_eq!(sent.len(), 1);
        match &sent[0] {
            ClientMessage::RemoteProjectionPush(request) => {
                assert_eq!(request.repo_id, repo_id);
                assert_eq!(request.branch, branch);
                assert_eq!(request.scope_nonce.get(), 9);
                assert_eq!(request.provider, RemoteProjectionProvider::WebDav);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    });
}

#[test]
fn command_without_runtime_context_fails_closed_without_source_control_notice() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        provide_context(test_session_client(ws.clone(), true, Some(9)));
        let (show_palette, set_show_palette) = signal(true);
        let commands = remote_projection_commands(Locale::En, set_show_palette);

        assert!(commands[0].availability.is_unavailable());
        assert_eq!(
            commands[0].availability.reason(),
            Some("Unavailable: current repository scope is not ready")
        );
        commands[0].action.run(());

        assert!(!show_palette.get_untracked());
        assert!(ws.drain_sent_for_test().is_empty());
    });
}

#[test]
fn push_admission_requires_valid_repo_and_exact_nonzero_handshake_scope() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let session = test_session_client(ws, true, Some(9));
        let repo_id = RepoId::new_v4().to_string();
        let rejected = [
            (None, 9, true, Some(9)),
            (Some("not-a-repo-id".to_string()), 9, true, Some(9)),
            (Some(repo_id.clone()), 0, true, Some(0)),
            (Some(repo_id.clone()), 9, false, Some(9)),
            (Some(repo_id.clone()), 9, true, None),
            (Some(repo_id.clone()), 9, true, Some(8)),
        ];

        for (current_repo_id, scope_nonce, ready, handshake_scope_nonce) in rejected {
            assert!(
                RemoteProjectionPushIntent::admit(
                    session.clone(),
                    current_repo_id,
                    None,
                    scope_nonce,
                    ready,
                    handshake_scope_nonce,
                )
                .is_none()
            );
        }
        assert!(
            RemoteProjectionPushIntent::admit(session, Some(repo_id), None, 9, true, Some(9),)
                .is_some()
        );
    });
}

fn test_session_client(
    ws: WsService,
    handshake_ready_value: bool,
    scope_nonce: Option<u64>,
) -> SessionClient {
    let (status_text, _) = signal(String::new());
    let (sync_banner, set_sync_banner) = signal(None::<String>);
    let (handshake_ready, _) = signal(handshake_ready_value);
    let (handshake_scope_nonce, _) = signal(scope_nonce);
    SessionClient {
        connection_status: ws.status,
        status_text: status_text.into(),
        sync_banner: sync_banner.into(),
        set_sync_banner,
        handshake_ready,
        handshake_scope_nonce,
        on_retry_peer_registration: Callback::new(|_| {}),
        ws,
    }
}

fn test_scope_client(
    repo_id: Option<String>,
    branch: Option<PeerId>,
    scope_nonce: u64,
) -> ScopeClient {
    let (current_doc, _) = signal(None::<DocId>);
    let (current_repo, set_current_repo) = signal(Some("repo".to_string()));
    let (current_repo_id, set_current_repo_id) = signal(repo_id);
    let (current_scope_nonce, _) = signal(scope_nonce);
    let (active_branch, set_active_branch) = signal(branch);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    let (shadow_repos, _) = signal(Vec::<String>::new());
    let (repo_list, _) = signal(Vec::<String>::new());
    let (repo_entries, _) = signal(Vec::<RepoListEntry>::new());
    let (removal_preview, _) = signal(None);
    let (is_spectator, _) = signal(false);
    ScopeClient {
        current_doc,
        current_repo,
        current_repo_id,
        current_scope_nonce,
        active_branch,
        set_active_branch,
        pending_repo_switch,
        on_switch_repo: noop::<RepoSwitchRequest>(),
        on_create_repo: noop::<String>(),
        on_rename_repo: noop::<RepoRenameRequest>(),
        on_remove_repo: noop::<RepoRemoveRequest>(),
        removal_preview,
        on_confirm_remove_repo: noop::<deve_core::models::RepoId>(),
        on_cancel_remove_repo: noop::<deve_core::models::RepoId>(),
        on_switch_branch: noop::<Option<String>>(),
        set_current_repo,
        set_current_repo_id,
        shadow_repos,
        on_list_shadows: noop::<()>(),
        repo_list,
        repo_entries,
        is_spectator: is_spectator.into(),
    }
}

fn noop<T>() -> Callback<T>
where
    T: 'static,
{
    Callback::new(|_| {})
}
