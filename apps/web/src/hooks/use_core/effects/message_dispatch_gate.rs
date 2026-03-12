use leptos::prelude::*;

use super::super::state::CoreSignals;

pub fn accepts_unscoped_update(signals: CoreSignals) -> bool {
    signals.pending_branch_switch.get_untracked().is_none()
        && signals.pending_repo_switch.get_untracked().is_none()
}

pub fn accepts_plugin_response(req_id: &str, signals: CoreSignals) -> bool {
    accepts_unscoped_update(signals)
        && signals
            .plugin_request_ids
            .get_untracked()
            .iter()
            .any(|id| id == req_id)
}

pub fn accepts_search_results(request_id: &str, signals: CoreSignals) -> bool {
    accepts_unscoped_update(signals)
        && signals.search_request_id.get_untracked().as_deref() == Some(request_id)
}

#[cfg(test)]
mod tests {
    use super::{accepts_plugin_response, accepts_search_results, accepts_unscoped_update};
    use crate::api::ConnectionStatus;
    use crate::hooks::use_core::PendingBranchTarget;
    use crate::hooks::use_core::state::init_signals;
    use leptos::prelude::*;

    #[test]
    fn rejects_unscoped_updates_while_repo_switch_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        let signals = init_signals(connection_status);
        signals.set_pending_repo_switch.set(Some("test".into()));
        assert!(!accepts_unscoped_update(signals));
    }

    #[test]
    fn rejects_unscoped_updates_while_branch_switch_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        let signals = init_signals(connection_status);
        signals
            .set_pending_branch_switch
            .set(Some(PendingBranchTarget::Local));
        assert!(!accepts_unscoped_update(signals));
    }

    #[test]
    fn rejects_search_results_when_request_id_is_stale() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        let signals = init_signals(connection_status);
        signals.set_search_request_id.set(Some("fresh".into()));
        assert!(!accepts_search_results("stale", signals));
        assert!(accepts_search_results("fresh", signals));
    }

    #[test]
    fn rejects_plugin_response_when_req_id_is_stale() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        let signals = init_signals(connection_status);
        signals.set_plugin_request_ids.set(vec!["fresh".into()]);
        assert!(!accepts_plugin_response("stale", signals));
        assert!(accepts_plugin_response("fresh", signals));
    }
}
