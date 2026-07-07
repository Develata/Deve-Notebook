use super::*;
use crate::api::ConnectionStatus;
use crate::hooks::use_core::state::init_signals;
use deve_core::models::RepoId;
use leptos::prelude::*;

fn init_runtime() -> (
    leptos::reactive::owner::Owner,
    crate::hooks::use_core::state::CoreSignals,
) {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    (runtime, init_signals(connection_status))
}

fn bind_local_scope(signals: crate::hooks::use_core::state::CoreSignals, repo_id: RepoId) {
    signals.set_current_repo_id.set(Some(repo_id.to_string()));
    signals.set_current_scope_nonce.set(7);
}

#[test]
fn unbound_runtime_sync_status_is_rejected_before_scope_exists() {
    let (_runtime, signals) = init_runtime();

    handle_sync_mode_status(None, None, None, Some(0), "manual".into(), signals);

    assert_eq!(signals.sync_mode.get_untracked(), "auto");
}

#[test]
fn sync_mode_status_updates_only_for_current_repo_scope() {
    let (_runtime, signals) = init_runtime();
    let repo_id = RepoId::new_v4();
    bind_local_scope(signals, repo_id);
    signals
        .set_sync_mode_request_id
        .set(Some("sync-mode-1".into()));

    handle_sync_mode_status(
        Some("sync-mode-1".into()),
        Some(repo_id),
        None,
        Some(7),
        "manual".into(),
        signals,
    );

    assert_eq!(signals.sync_mode.get_untracked(), "manual");
    assert_eq!(signals.sync_mode_request_id.get_untracked(), None);
}

#[test]
fn pending_ops_info_rejects_stale_scope() {
    let (_runtime, signals) = init_runtime();
    let repo_id = RepoId::new_v4();
    bind_local_scope(signals, repo_id);
    signals
        .set_pending_ops_request_id
        .set(Some("pending-1".into()));

    handle_pending_ops_info(
        Some("pending-1".into()),
        Some(repo_id),
        None,
        Some(6),
        2,
        vec![PendingOpsPreview::new("a".into(), "b".into(), "c".into())],
        signals,
    );

    assert_eq!(signals.pending_ops_count.get_untracked(), 0);
    assert_eq!(
        signals.pending_ops_request_id.get_untracked().as_deref(),
        Some("pending-1")
    );
}

#[test]
fn merge_complete_clears_pending_ops_only_for_current_repo_scope() {
    let (_runtime, signals) = init_runtime();
    let repo_id = RepoId::new_v4();
    let other_repo_id = RepoId::new_v4();
    bind_local_scope(signals, repo_id);
    signals.set_pending_ops_count.set(3);
    signals
        .set_pending_ops_previews
        .set(vec![PendingOpsPreview::new(
            "a".into(),
            "b".into(),
            "c".into(),
        )]);

    handle_merge_complete(Some(other_repo_id), None, Some(7), 3, signals);

    assert_eq!(signals.pending_ops_count.get_untracked(), 3);
    assert_eq!(signals.pending_ops_previews.get_untracked().len(), 1);

    handle_merge_complete(Some(repo_id), None, Some(7), 3, signals);

    assert_eq!(signals.pending_ops_count.get_untracked(), 0);
    assert!(signals.pending_ops_previews.get_untracked().is_empty());
}
