use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::types::SwitchScopeSignals;
use crate::hooks::use_core::{PendingBranchSwitch, PendingRepoSwitch, RepoSwitchRequest};
use crate::i18n::Locale;
use crate::runtime::document::pending::PendingLocalEdits;
use deve_core::models::RepoId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{Callable, GetUntracked, RwSignal, signal};

use super::build_switch_repo_callback;

#[test]
fn switch_repo_exact_sends_repo_id_even_when_display_name_matches_current() {
    let owner = leptos::reactive::owner::Owner::new();
    owner.with(|| {
        let target_repo_id = RepoId::new_v4();
        let other_repo_id = RepoId::new_v4();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let (sync_banner, set_sync_banner) = signal(None::<String>);
        let locale = RwSignal::new(Locale::Zh);
        let signals = switch_signals(
            Some("display".to_string()),
            Some(other_repo_id.to_string()),
            7,
        );
        let callback = build_switch_repo_callback(ws.clone(), locale, signals, set_sync_banner);

        callback.run(RepoSwitchRequest::exact(
            "display".to_string(),
            target_repo_id,
        ));

        assert_eq!(sync_banner.get_untracked(), None);
        let pending = signals
            .pending_repo_switch
            .get_untracked()
            .expect("pending repo switch");
        assert_eq!(pending.expected_name(), "display");
        assert!(pending.switch_nonce > 7);
        match ws.drain_sent_for_test().as_slice() {
            [
                ClientMessage::SwitchRepoExact {
                    repo_id,
                    switch_nonce,
                },
            ] => {
                assert_eq!(*repo_id, target_repo_id);
                assert_eq!(*switch_nonce, Some(pending.switch_nonce));
            }
            other => panic!("expected one SwitchRepoExact message, got {other:?}"),
        }
    });
}

fn switch_signals(
    current_repo_value: Option<String>,
    current_repo_id_value: Option<String>,
    current_scope_nonce_value: u64,
) -> SwitchScopeSignals {
    let (current_doc, _) = signal(None);
    let (pending_local_edits, _) = signal(PendingLocalEdits::new());
    let (_, set_pending_navigation) = signal(None);
    let (current_repo, _) = signal(current_repo_value);
    let (current_repo_id, _) = signal(current_repo_id_value);
    let (current_scope_nonce, _) = signal(current_scope_nonce_value);
    let (active_branch, _) = signal(None);
    let (pending_branch_switch, set_pending_branch_switch) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<PendingRepoSwitch>);
    let (_, set_handshake_ready) = signal(true);
    let (_, set_handshake_scope_nonce) = signal(Some(current_scope_nonce_value));

    SwitchScopeSignals {
        current_doc,
        pending_local_edits,
        set_pending_navigation,
        current_repo,
        current_repo_id,
        current_scope_nonce,
        active_branch,
        pending_branch_switch,
        pending_repo_switch,
        set_handshake_ready,
        set_handshake_scope_nonce,
        set_pending_branch_switch,
        set_pending_repo_switch,
    }
}
