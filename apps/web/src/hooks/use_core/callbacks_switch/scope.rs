//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use leptos::prelude::{GetUntracked, Set};

use super::super::types::SwitchScopeSignals;

pub fn prepare_scope_switch(ws: &WsService, signals: SwitchScopeSignals) {
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals.set_handshake_scope_nonce.set(None);
}

pub fn can_start_scope_switch(signals: SwitchScopeSignals) -> bool {
    signals.pending_branch_switch.get_untracked().is_none()
        && signals.pending_repo_switch.get_untracked().is_none()
}

#[cfg(test)]
mod tests {
    use super::can_start_scope_switch;
    use crate::hooks::use_core::{
        PendingBranchSwitch, PendingBranchTarget, PendingRepoSwitch, SwitchScopeSignals,
    };
    use leptos::prelude::*;

    #[test]
    fn blocks_reentrant_scope_switches() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_doc, _) = signal(None);
        let (pending_local_edits, _) = signal(Default::default());
        let (_, set_pending_navigation) = signal(None);
        let (current_repo, _) = signal(Some("default".to_string()));
        let (current_repo_id, _) = signal(Some(uuid::Uuid::new_v4().to_string()));
        let (current_scope_nonce, _) = signal(7u64);
        let (active_branch, _) = signal(None::<deve_core::models::PeerId>);
        let (_, set_handshake_ready) = signal(false);
        let (_, set_handshake_scope_nonce) = signal(None::<u64>);
        let (pending_branch_switch, set_pending_branch_switch) = signal(Some(
            PendingBranchSwitch::new(PendingBranchTarget::Local, 1),
        ));
        let (pending_repo_switch, set_pending_repo_switch) = signal(None::<PendingRepoSwitch>);

        assert!(!can_start_scope_switch(SwitchScopeSignals {
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
        }));

        set_pending_branch_switch.set(None);
        set_pending_repo_switch.set(Some(PendingRepoSwitch::switch(
            "repo-2",
            uuid::Uuid::nil(),
            9,
        )));

        assert!(!can_start_scope_switch(SwitchScopeSignals {
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
        }));
    }
}
