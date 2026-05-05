//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
//! `CoreSignals` 初始化工厂。

#[path = "state_init/build.rs"]
mod build;
#[path = "state_init/docs.rs"]
mod docs;
#[path = "state_init/repo.rs"]
mod repo;
#[path = "state_init/runtime.rs"]
mod runtime;
#[path = "state_init/source_control.rs"]
mod source_control;

use crate::api::ConnectionStatus;
use leptos::prelude::*;

use super::state::CoreSignals;

/// 初始化所有核心信号。
///
/// Invariant:
/// - `is_spectator` 在远端分支、降级存储或断连时必须为真。
pub fn init_signals(connection_status: ReadSignal<ConnectionStatus>) -> CoreSignals {
    let docs = docs::init_doc_signals();
    let repo = repo::init_repo_signals();
    let runtime = runtime::init_runtime_signals();
    let source_control = source_control::init_source_control_signals();
    build::assemble_core_signals(connection_status, docs, repo, runtime, source_control)
}

#[cfg(test)]
mod tests {
    use super::init_signals;
    use crate::api::ConnectionStatus;
    use leptos::prelude::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn disconnected_lockdown_marks_core_as_spectator() {
        let runtime = Owner::new();
        runtime.set();
        let (connection_status, set_connection_status) = signal(ConnectionStatus::Disconnected);
        let signals = init_signals(connection_status);

        assert!(signals.is_spectator.get_untracked());

        set_connection_status.set(ConnectionStatus::Connecting);
        assert!(signals.is_spectator.get_untracked());
    }

    #[test]
    fn disconnected_lockdown_releases_when_connection_is_ready() {
        let runtime = Owner::new();
        runtime.set();
        let (connection_status, set_connection_status) = signal(ConnectionStatus::Disconnected);
        let signals = init_signals(connection_status);

        assert!(signals.is_spectator.get_untracked());

        set_connection_status.set(ConnectionStatus::Connected);
        assert!(!signals.is_spectator.get_untracked());
    }
}
