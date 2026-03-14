use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::navigation::{NavigationTarget, guard_navigation};
use super::switch_nonce::next_switch_nonce;
use super::types::{PendingBranchTarget, SwitchScopeSignals};

pub struct SwitchCallbacks {
    pub on_switch_branch: Callback<Option<String>>,
    pub on_switch_repo: Callback<String>,
}

pub fn create_switch_callbacks(ws: &WsService, signals: SwitchScopeSignals) -> SwitchCallbacks {
    let ws_branch = ws.clone();
    let on_switch_branch = Callback::new(move |peer_id: Option<String>| {
        if !can_start_scope_switch(signals) {
            leptos::logging::warn!("忽略分支切换: 仍有 scope switch 挂起");
            return;
        }
        let same_branch = signals
            .active_branch
            .get_untracked()
            .as_ref()
            .map(PeerId::as_str)
            == peer_id.as_deref();
        if same_branch {
            return;
        }
        let target_peer = peer_id.clone();
        let ws_branch_action = ws_branch.clone();
        let action = Callback::new(move |_: ()| {
            let switch_nonce = next_switch_nonce();
            let pending = target_peer
                .clone()
                .map(PendingBranchTarget::Shadow)
                .unwrap_or(PendingBranchTarget::Local);
            prepare_scope_switch(&ws_branch_action, signals);
            signals.set_pending_branch_switch.set(Some(pending));
            signals
                .set_pending_branch_switch_nonce
                .set(Some(switch_nonce));
            signals.set_pending_repo_switch.set(None);
            signals.set_pending_repo_switch_nonce.set(None);
            ws_branch_action.send(ClientMessage::SwitchBranch {
                peer_id: target_peer.clone(),
                switch_nonce: Some(switch_nonce),
            });
        });
        let _ = guard_navigation(
            signals.current_doc.get_untracked(),
            &signals.pending_local_edits.get_untracked(),
            signals.set_pending_navigation,
            NavigationTarget::Branch,
            action,
        );
    });

    let ws_repo = ws.clone();
    let on_switch_repo = Callback::new(move |name: String| {
        if !can_start_scope_switch(signals) {
            leptos::logging::warn!("忽略仓库切换: 仍有 scope switch 挂起");
            return;
        }
        if signals.current_repo.get_untracked().as_deref() == Some(name.as_str()) {
            return;
        }
        let target_repo = name.clone();
        let ws_repo_action = ws_repo.clone();
        let action = Callback::new(move |_: ()| {
            let switch_nonce = next_switch_nonce();
            prepare_scope_switch(&ws_repo_action, signals);
            signals
                .set_pending_repo_switch
                .set(Some(target_repo.clone()));
            signals
                .set_pending_repo_switch_nonce
                .set(Some(switch_nonce));
            ws_repo_action.send(ClientMessage::SwitchRepo {
                name: target_repo.clone(),
                switch_nonce: Some(switch_nonce),
            });
        });
        let _ = guard_navigation(
            signals.current_doc.get_untracked(),
            &signals.pending_local_edits.get_untracked(),
            signals.set_pending_navigation,
            NavigationTarget::Repo,
            action,
        );
    });

    SwitchCallbacks {
        on_switch_branch,
        on_switch_repo,
    }
}

fn prepare_scope_switch(ws: &WsService, signals: SwitchScopeSignals) {
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals.set_handshake_scope_nonce.set(None);
}

fn can_start_scope_switch(signals: SwitchScopeSignals) -> bool {
    signals.pending_branch_switch.get_untracked().is_none()
        && signals.pending_repo_switch.get_untracked().is_none()
        && signals
            .pending_branch_switch_nonce
            .get_untracked()
            .is_none()
        && signals.pending_repo_switch_nonce.get_untracked().is_none()
}

#[cfg(test)]
mod tests {
    use super::can_start_scope_switch;
    use crate::hooks::use_core::SwitchScopeSignals;
    use leptos::prelude::*;

    #[test]
    fn blocks_reentrant_scope_switches() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_doc, _) = signal(None);
        let (pending_local_edits, _) = signal(Default::default());
        let (_, set_pending_navigation) = signal(None);
        let (current_repo, _) = signal(Some("default".to_string()));
        let (active_branch, _) = signal(None::<deve_core::models::PeerId>);
        let (_, set_handshake_ready) = signal(false);
        let (_, set_handshake_scope_nonce) = signal(None::<u64>);
        let (pending_branch_switch, set_pending_branch_switch) = signal(None);
        let (pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(Some(1u64));
        let (pending_repo_switch, set_pending_repo_switch) = signal(None::<String>);
        let (pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(None::<u64>);

        assert!(!can_start_scope_switch(SwitchScopeSignals {
            current_doc,
            pending_local_edits,
            set_pending_navigation,
            current_repo,
            active_branch,
            pending_branch_switch,
            pending_branch_switch_nonce,
            pending_repo_switch,
            pending_repo_switch_nonce,
            set_handshake_ready,
            set_handshake_scope_nonce,
            set_pending_branch_switch,
            set_pending_branch_switch_nonce,
            set_pending_repo_switch,
            set_pending_repo_switch_nonce,
        }));

        set_pending_repo_switch.set(Some("repo-2".to_string()));
        set_pending_branch_switch_nonce.set(None);
        set_pending_repo_switch_nonce.set(Some(9));

        assert!(!can_start_scope_switch(SwitchScopeSignals {
            current_doc,
            pending_local_edits,
            set_pending_navigation,
            current_repo,
            active_branch,
            pending_branch_switch,
            pending_branch_switch_nonce,
            pending_repo_switch,
            pending_repo_switch_nonce,
            set_handshake_ready,
            set_handshake_scope_nonce,
            set_pending_branch_switch,
            set_pending_branch_switch_nonce,
            set_pending_repo_switch,
            set_pending_repo_switch_nonce,
        }));
    }
}
