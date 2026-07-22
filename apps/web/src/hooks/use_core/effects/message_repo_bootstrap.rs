//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::PendingRepoSwitch;
use crate::hooks::use_core::scope_prefs::clear_scope_pref;
use deve_core::protocol::{ClientMessage, RepoListEntry};
use leptos::prelude::{GetUntracked, Set};

use super::super::state::CoreSignals;
use super::super::switch_nonce::next_switch_nonce_after;

pub fn maybe_switch_to_first_repo(entries: &[RepoListEntry], ws: &WsService, signals: CoreSignals) {
    let Some(target_repo) = bootstrap_repo_target(entries, signals) else {
        clear_stale_unbound_scope(entries, signals);
        return;
    };

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
            target_repo.display_alias.clone(),
            target_repo.repo_id,
            switch_nonce,
        )));
    ws.send(ClientMessage::SwitchRepoExact {
        repo_id: target_repo.repo_id,
        switch_nonce: Some(switch_nonce),
    });
}

fn bootstrap_repo_target(
    entries: &[RepoListEntry],
    signals: CoreSignals,
) -> Option<&RepoListEntry> {
    if entries.is_empty()
        || signals.explicit_repo_selection_required.get_untracked()
        || signals.active_branch.get_untracked().is_some()
        || signals.pending_branch_switch.get_untracked().is_some()
        || signals.pending_repo_switch.get_untracked().is_some()
    {
        return None;
    }

    match (
        signals.current_repo.get_untracked(),
        signals.current_repo_id.get_untracked(),
    ) {
        (None, Some(restored_repo_id)) => restored_repo_id
            .parse::<deve_core::models::RepoId>()
            .ok()
            .and_then(|repo_id| entries.iter().find(|entry| entry.repo_id == repo_id)),
        (None, None) => entries.first(),
        _ => None,
    }
}

fn clear_stale_unbound_scope(entries: &[RepoListEntry], signals: CoreSignals) {
    let stale = signals.current_repo.get_untracked().is_none()
        && signals
            .current_repo_id
            .get_untracked()
            .and_then(|value| value.parse::<deve_core::models::RepoId>().ok())
            .is_some_and(|repo_id| entries.iter().all(|entry| entry.repo_id != repo_id));
    if stale {
        clear_scope_pref();
        signals.set_current_repo_id.set(None);
        signals.set_explicit_repo_selection_required.set(true);
    }
}

#[cfg(test)]
mod tests {
    use super::bootstrap_repo_target;
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

        assert_eq!(
            bootstrap_repo_target(&repos, signals).map(|entry| entry.display_alias.as_str()),
            Some("default")
        );

        signals
            .set_pending_repo_switch
            .set(Some(PendingRepoSwitch::switch(
                "default",
                uuid::Uuid::nil(),
                1,
            )));
        assert!(bootstrap_repo_target(&repos, signals).is_none());
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

        assert!(bootstrap_repo_target(&repos, signals).is_none());
    }

    #[test]
    fn explicit_selection_blocker_prevents_first_repo_autobind() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Connected).0);
        let repos = repo_entries(&["default", "test"]);

        signals.set_explicit_repo_selection_required.set(true);

        assert!(bootstrap_repo_target(&repos, signals).is_none());
    }

    #[test]
    fn exact_repo_restore_resolves_through_the_current_backend_list() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Connected).0);
        let repos = repo_entries(&["first", "remembered"]);
        signals
            .set_current_repo_id
            .set(Some(repos[1].repo_id.to_string()));

        assert_eq!(
            bootstrap_repo_target(&repos, signals).map(|entry| entry.repo_id),
            Some(repos[1].repo_id)
        );
    }

    #[test]
    fn missing_exact_repo_restore_stays_unbound() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Connected).0);
        let repos = repo_entries(&["current"]);
        signals
            .set_current_repo_id
            .set(Some(uuid::Uuid::new_v4().to_string()));

        assert!(bootstrap_repo_target(&repos, signals).is_none());
    }
}
