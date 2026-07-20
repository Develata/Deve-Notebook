//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::handlers::switcher::{handle_switch_branch, handle_switch_repo};
use super::switcher_test_support::{browser_session, build_state, unicast_channel};
use deve_core::protocol::{ServerErrorCode, ServerMessage};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_from_missing_shadow_without_repo_hint_does_not_silently_self_heal(
) -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(90);
    session.switch_branch(Some("missing-shadow".into()));

    handle_switch_branch(&state, &ch, &mut session, None, Some(91)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(switch_nonce, Some(91));
            assert!(error
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("Remote branch not available:")));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_repo_on_missing_shadow_branch_reports_scope_invalid_and_clears_remote_binding(
) -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(91);
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), None);

    handle_switch_repo(&state, &ch, &mut session, "ghost".into(), None, Some(92)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(switch_nonce, Some(92));
            assert!(error
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("Remote branch not available:")));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_from_missing_shadow_with_stale_runtime_binding_clears_all_scope(
) -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(98);
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), Some(default_id));
    session.set_active_db(local_handle);
    session.set_authenticated(deve_core::models::PeerId::new("stale-peer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(42);

    handle_switch_repo(&state, &ch, &mut session, "ghost".into(), None, Some(99)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(switch_nonce, Some(99));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
