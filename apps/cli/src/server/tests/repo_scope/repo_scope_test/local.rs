//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::support::build_state;
use crate::server::{
    repo_scope::{bootstrap_local_repo, resolve_session_repo, resolve_session_repo_and_sync},
    runtime::watcher_runtime::{RepoMountState, WatcherRuntimeView},
    session::WsSession,
    source_control_grants::{AuthSessionId, SourceControlGrantBranch},
};
use deve_core::models::PeerId;

#[test]
fn resolve_session_repo_rejects_stale_local_repo_id_mismatch() -> anyhow::Result<()> {
    let (_dir, state, default_id, _test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));
    let err = resolve_session_repo(&state, &session).expect_err("stale local repo must fail");
    assert!(err
        .to_string()
        .contains("Local repository selector not resolved"));
    assert_eq!(session.active_repo.as_deref(), Some("test"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[test]
fn resolve_session_repo_and_sync_rejects_stale_local_repo_id_mismatch() -> anyhow::Result<()> {
    let (_dir, state, default_id, _test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));

    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("stale local repo must fail before syncing session");
    assert!(err
        .to_string()
        .contains("Local repository selector not resolved"));
    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[test]
fn resolve_session_repo_and_sync_clears_stale_runtime_binding_after_selector_repair(
) -> anyhow::Result<()> {
    let (_dir, state, default_id, test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo(test_id.to_string(), Some(test_id));
    session.set_active_db(state.repo.open_database(None, &default_id.to_string())?);
    session.set_authenticated(PeerId::new("stale-writer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(17);
    session.set_writer_identity(default_id, PeerId::new("stale-writer"), 17);

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;
    assert_eq!(resolved.repo_name, test_id.to_string());
    assert_eq!(resolved.session_name, test_id.to_string());
    assert_eq!(resolved.repo_id, test_id);
    assert_eq!(
        session.active_repo.as_deref(),
        Some(test_id.to_string().as_str())
    );
    assert_eq!(session.active_repo_id, Some(test_id));
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    assert!(session.writer_identity.is_none());
    Ok(())
}

#[test]
fn repo_scope_runtime_cleanup_revokes_source_control_write_grant() -> anyhow::Result<()> {
    let (_dir, state, default_id, test_id) = build_state()?;
    let auth_session_id = AuthSessionId::for_test("repo-scope-runtime-cleanup");
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.bind_auth_session(auth_session_id.clone());
    session.switch_repo(test_id.to_string(), Some(test_id));
    session.set_active_db(state.repo.open_database(None, &default_id.to_string())?);
    session.set_authenticated(PeerId::new("stale-writer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(17);
    session.set_writer_identity(default_id, PeerId::new("stale-writer"), 17);
    state
        .source_control_write_grants()
        .grant(
            auth_session_id.clone(),
            default_id,
            SourceControlGrantBranch::Local,
            PeerId::new("stale-writer"),
            17,
        )
        .expect("source-control write grant");
    assert!(state
        .source_control_write_grants()
        .authorize_browser_local(&auth_session_id, default_id, 17)
        .is_ok());

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;
    assert_eq!(resolved.repo_id, test_id);
    assert!(session.writer_identity.is_none());

    state
        .source_control_write_grants()
        .authorize_browser_local(&auth_session_id, default_id, 17)
        .expect_err("repo scope cleanup must revoke stale source control write grant");
    Ok(())
}

#[test]
fn resolve_session_repo_rejects_local_name_recovery_from_stale_uuid() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("stale-name".into(), Some(test_id));
    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("local stale selector must fail closed");
    assert!(err
        .to_string()
        .contains("Local repository selector not resolved"));
    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[test]
fn resolve_session_repo_rejects_unrecoverable_stale_local_repo_name() -> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo("stale-name".into(), None);
    let err = resolve_session_repo(&state, &session).expect_err("stale local repo must fail");
    assert!(err
        .to_string()
        .contains("Local repository selector not resolved"));
    Ok(())
}

#[test]
fn bootstrap_local_repo_requires_explicit_selection_when_multiple_local_repos_exist(
) -> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let session = WsSession::new();
    let err = bootstrap_local_repo(&state, &session).expect_err("multi repo bootstrap must fail");
    assert!(err.to_string().contains("Active repository not selected"));
    Ok(())
}

#[test]
fn watcher_server_isolation_default_scope_uses_the_only_mounted_repo() -> anyhow::Result<()> {
    let (_dir, state, default_id, test_id) = build_state()?;
    let view =
        WatcherRuntimeView::with_state_for_test(default_id, 1, RepoMountState::Mounted);
    view.insert_state_for_test(test_id, 1, RepoMountState::Failed);
    state.set_watcher_runtime_view_for_test(view);

    let resolved = bootstrap_local_repo(&state, &WsSession::new())?;

    assert_eq!(resolved.repo_id, default_id);
    Ok(())
}

#[test]
fn watcher_server_isolation_default_scope_rejects_zero_mounted_repos() -> anyhow::Result<()> {
    let (_dir, state, default_id, test_id) = build_state()?;
    let view = WatcherRuntimeView::with_state_for_test(default_id, 1, RepoMountState::Failed);
    view.insert_state_for_test(test_id, 1, RepoMountState::Failed);
    state.set_watcher_runtime_view_for_test(view);

    let error = bootstrap_local_repo(&state, &WsSession::new())
        .expect_err("failed watcher slots must not become the default scope");

    assert!(error.to_string().contains("No mounted local repository"));
    Ok(())
}
