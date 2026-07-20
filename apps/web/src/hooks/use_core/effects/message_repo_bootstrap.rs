//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::PendingRepoSwitch;
use deve_core::protocol::{ClientMessage, RepoListEntry};
use leptos::prelude::{GetUntracked, Set};

use super::super::state::CoreSignals;
use super::super::switch_nonce::next_switch_nonce_after;

pub fn maybe_switch_to_first_repo(entries: &[RepoListEntry], ws: &WsService, signals: CoreSignals) {
    let Some(first_repo) = entries.first() else {
        return;
    };
    if !should_auto_switch_to_first_repo(entries, signals) {
        return;
    }

    let Some(switch_nonce) = next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
    else {
        return;
    };
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    signals.set_handshake_scope_nonce.set(None);
    signals
        .set_pending_repo_switch
        .set(Some(PendingRepoSwitch::switch(
            first_repo.display_alias.clone(),
            first_repo.repo_id,
            switch_nonce,
        )));
    ws.send(ClientMessage::SwitchRepoExact {
        repo_id: first_repo.repo_id,
        switch_nonce: Some(switch_nonce),
    });
}

fn should_auto_switch_to_first_repo(entries: &[RepoListEntry], signals: CoreSignals) -> bool {
    !entries.is_empty()
        && !signals.explicit_repo_selection_required.get_untracked()
        && signals.current_repo.get_untracked().is_none()
        && signals.current_repo_id.get_untracked().is_none()
        && signals.active_branch.get_untracked().is_none()
        && signals.pending_branch_switch.get_untracked().is_none()
        && signals.pending_repo_switch.get_untracked().is_none()
}

#[cfg(test)]
mod tests {
    use super::should_auto_switch_to_first_repo;
    use crate::hooks::use_core::PendingRepoSwitch;
    use crate::hooks::use_core::state::init_signals;
    use leptos::prelude::*;

    fn repo_entries(names: &[&str]) -> Vec<deve_core::protocol::RepoListEntry> {
        names
            .iter()
            .map(|name| deve_core::protocol::RepoListEntry {
                repo_id: uuid::Uuid::new_v4(),
                display_alias: (*name).to_string(),
                alias_revision: 0,
                readiness: deve_core::protocol::RepoReadiness::Mounted,
            })
            .collect()
    }

    #[test]
    fn auto_switch_runs_only_for_unbound_scope_with_repos() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Connected).0);
        let repos = repo_entries(&["default", "test"]);

        assert!(should_auto_switch_to_first_repo(&repos, signals));

        signals
            .set_pending_repo_switch
            .set(Some(PendingRepoSwitch::switch(
                "default",
                uuid::Uuid::nil(),
                1,
            )));
        assert!(!should_auto_switch_to_first_repo(&repos, signals));
    }

    #[test]
    fn auto_switch_skips_when_repo_is_already_bound() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Connected).0);
        let repos = repo_entries(&["default"]);

        signals.set_current_repo.set(Some("default".to_string()));
        signals
            .set_current_repo_id
            .set(Some(uuid::Uuid::new_v4().to_string()));

        assert!(!should_auto_switch_to_first_repo(&repos, signals));
    }

    #[test]
    fn explicit_selection_blocker_prevents_first_repo_autobind() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Connected).0);
        let repos = repo_entries(&["default", "test"]);

        signals.set_explicit_repo_selection_required.set(true);

        assert!(!should_auto_switch_to_first_repo(&repos, signals));
    }
}
