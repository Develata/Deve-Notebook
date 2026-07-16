//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::ConnectionStatus;
use leptos::prelude::*;

use super::super::repo::RepoSignals;

pub fn build_is_spectator(
    connection_status: ReadSignal<ConnectionStatus>,
    repo: RepoSignals,
) -> Memo<bool> {
    Memo::new(move |_| {
        let disconnected = connection_status.get() != ConnectionStatus::Connected;
        repo.active_branch.get().is_some()
            || repo.degraded_sync_mode.get().is_some()
            || disconnected
    })
}
