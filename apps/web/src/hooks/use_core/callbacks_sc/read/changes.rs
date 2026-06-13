//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_read_scope_nonce;
use crate::hooks::use_core::write_gate::{
    RepoWriteSignals, repo_source_control_read_block_untracked,
};
use deve_core::protocol::ClientMessage;
use leptos::prelude::{Callback, Set, WriteSignal};

use super::{SourceControlScopeSignals, log_blocked_sc_read};

pub(super) fn create_get_changes_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    read_gate: RepoWriteSignals,
    set_request_id: WriteSignal<Option<String>>,
) -> Callback<()> {
    let ws = ws.clone();
    Callback::new(move |_: ()| {
        if let Some(block) = repo_source_control_read_block_untracked(&ws, read_gate) {
            log_blocked_sc_read("GetChanges", "working tree", block);
            return;
        }
        let Some(scope_nonce) = source_control_read_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        set_request_id.set(Some(request_id.clone()));
        ws.send(ClientMessage::GetChanges {
            request_id,
            scope_nonce: Some(scope_nonce),
        });
    })
}

pub(super) fn create_get_history_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    read_gate: RepoWriteSignals,
    set_request_id: WriteSignal<Option<String>>,
) -> Callback<u32> {
    let ws = ws.clone();
    Callback::new(move |limit: u32| {
        if let Some(block) = repo_source_control_read_block_untracked(&ws, read_gate) {
            log_blocked_sc_read("GetCommitHistory", &format!("limit={limit}"), block);
            return;
        }
        let Some(scope_nonce) = source_control_read_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        set_request_id.set(Some(request_id.clone()));
        ws.send(ClientMessage::GetCommitHistory {
            request_id,
            limit,
            scope_nonce: Some(scope_nonce),
        });
    })
}

#[cfg(test)]
mod tests {
    use super::{create_get_changes_callback, create_get_history_callback};
    use crate::api::{ConnectionStatus, WsService};
    use crate::hooks::use_core::PendingBranchTarget;
    use crate::hooks::use_core::callbacks_sc::SourceControlScopeSignals;
    use crate::hooks::use_core::write_gate::RepoWriteSignals;
    use deve_core::models::PeerId;
    use deve_core::protocol::ClientMessage;
    use leptos::prelude::{Callable, GetUntracked, signal};

    fn remote_read_signals() -> (
        leptos::reactive::owner::Owner,
        WsService,
        SourceControlScopeSignals,
        RepoWriteSignals,
    ) {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let (current_repo_id, _) = signal(Some("repo-a".to_string()));
        let (active_branch, _) = signal(Some(PeerId::new("peer-a")));
        let (current_scope_nonce, _) = signal(11u64);
        let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
        let (pending_repo_switch, _) = signal(None::<String>);
        let (load_state, _) = signal("ready".to_string());
        let (is_spectator, _) = signal(true);
        let (handshake_ready, _) = signal(false);
        let scope = SourceControlScopeSignals {
            current_repo_id,
            active_branch,
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
        };
        let gate = RepoWriteSignals {
            load_state,
            is_spectator: is_spectator.into(),
            handshake_ready,
            current_repo_id,
            current_scope_nonce,
            active_branch,
            pending_branch_switch,
            pending_repo_switch,
        };
        (runtime, ws, scope, gate)
    }

    #[test]
    fn changes_read_gate_allows_remote_branch_spectator_reads() {
        let (_runtime, ws, scope, gate) = remote_read_signals();
        let (request_id, set_request_id) = signal(None::<String>);
        let callback = create_get_changes_callback(&ws, scope, gate, set_request_id);

        callback.run(());

        let request_id = request_id.get_untracked().expect("changes request");
        let sent = ws.drain_sent_for_test();
        assert_eq!(sent.len(), 1);
        match &sent[0] {
            ClientMessage::GetChanges {
                request_id: sent_request_id,
                scope_nonce,
            } => {
                assert_eq!(sent_request_id, &request_id);
                assert_eq!(*scope_nonce, Some(11));
            }
            other => panic!("expected GetChanges, got {other:?}"),
        }
    }

    #[test]
    fn history_read_gate_allows_remote_branch_spectator_reads() {
        let (_runtime, ws, scope, gate) = remote_read_signals();
        let (request_id, set_request_id) = signal(None::<String>);
        let callback = create_get_history_callback(&ws, scope, gate, set_request_id);

        callback.run(32);

        let request_id = request_id.get_untracked().expect("history request");
        let sent = ws.drain_sent_for_test();
        assert_eq!(sent.len(), 1);
        match &sent[0] {
            ClientMessage::GetCommitHistory {
                request_id: sent_request_id,
                limit,
                scope_nonce,
            } => {
                assert_eq!(sent_request_id, &request_id);
                assert_eq!(*limit, 32);
                assert_eq!(*scope_nonce, Some(11));
            }
            other => panic!("expected GetCommitHistory, got {other:?}"),
        }
    }
}
