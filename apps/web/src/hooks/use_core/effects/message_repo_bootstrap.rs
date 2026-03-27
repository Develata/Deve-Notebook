use crate::api::WsService;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set};

use super::super::state::CoreSignals;
use super::super::switch_nonce::next_switch_nonce;

pub fn maybe_switch_to_first_repo(repos: &[String], ws: &WsService, signals: CoreSignals) {
    let Some(first_repo) = repos.first().cloned() else {
        return;
    };
    if !should_auto_switch_to_first_repo(repos, signals) {
        return;
    }

    let switch_nonce = next_switch_nonce();
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals.set_handshake_scope_nonce.set(None);
    signals
        .set_pending_repo_switch
        .set(Some(first_repo.clone()));
    signals
        .set_pending_repo_switch_nonce
        .set(Some(switch_nonce));
    ws.send(ClientMessage::SwitchRepo {
        name: first_repo,
        switch_nonce: Some(switch_nonce),
    });
}

fn should_auto_switch_to_first_repo(repos: &[String], signals: CoreSignals) -> bool {
    !repos.is_empty()
        && signals.current_repo.get_untracked().is_none()
        && signals.current_repo_id.get_untracked().is_none()
        && signals.active_branch.get_untracked().is_none()
        && signals.pending_branch_switch.get_untracked().is_none()
        && signals
            .pending_branch_switch_nonce
            .get_untracked()
            .is_none()
        && signals.pending_repo_switch.get_untracked().is_none()
        && signals.pending_repo_switch_nonce.get_untracked().is_none()
}

#[cfg(test)]
mod tests {
    use super::should_auto_switch_to_first_repo;
    use crate::hooks::use_core::state::init_signals;
    use leptos::prelude::*;

    #[test]
    fn auto_switch_runs_only_for_unbound_scope_with_repos() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Connected).0);
        let repos = vec!["default".to_string(), "test".to_string()];

        assert!(should_auto_switch_to_first_repo(&repos, signals));

        signals
            .set_pending_repo_switch
            .set(Some("default".to_string()));
        assert!(!should_auto_switch_to_first_repo(&repos, signals));
    }

    #[test]
    fn auto_switch_skips_when_repo_is_already_bound() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Connected).0);
        let repos = vec!["default".to_string()];

        signals.set_current_repo.set(Some("default".to_string()));
        signals
            .set_current_repo_id
            .set(Some(uuid::Uuid::new_v4().to_string()));

        assert!(!should_auto_switch_to_first_repo(&repos, signals));
    }
}
