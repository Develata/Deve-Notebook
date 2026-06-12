//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::handlers::sync::{
    handle_register_writer, handle_sync_request, handle_sync_snapshot_request,
};
use super::source_control_grants::{AuthSessionId, SourceControlGrantBranch};
use super::sync_scope_cleanup_test_support::{
    assert_runtime_binding_cleared, browser_session_without_sync_scope, build_state,
    recv_protocol_error, try_recv_protocol_error, unicast_channel,
};
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_request_rejects_missing_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session_without_sync_scope(&state, repo_id, 17)?;

    handle_sync_request(&state, &ch, &mut session, repo_id, vec![]).await;

    let (error, scope_nonce) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(scope_nonce, Some(17));
    assert_runtime_binding_cleared(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_request_rejects_stale_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session_without_sync_scope(&state, repo_id, 23)?;
    session.set_sync_scope_nonce(19);

    handle_sync_request(&state, &ch, &mut session, repo_id, vec![]).await;

    let (error, scope_nonce) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScStaleScope);
    assert_eq!(scope_nonce, Some(23));
    assert_runtime_binding_cleared(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_guard_scope_cleanup_revokes_source_control_write_grant() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let auth_session_id = AuthSessionId::for_test("sync-guard-cleanup");
    let mut session = browser_session_without_sync_scope(&state, repo_id, 23)?;
    session.bind_auth_session(auth_session_id.clone());
    session.set_sync_scope_nonce(19);
    state.source_control_write_grants().grant(
        auth_session_id.clone(),
        repo_id,
        SourceControlGrantBranch::Local,
        PeerId::new("browser"),
        19,
    );
    assert!(
        state
            .source_control_write_grants()
            .authorize_browser_local(&auth_session_id, repo_id, 19)
            .is_ok()
    );

    handle_sync_request(&state, &ch, &mut session, repo_id, vec![]).await;

    let (error, scope_nonce) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScStaleScope);
    assert_eq!(scope_nonce, Some(23));
    assert_runtime_binding_cleared(&session);
    state
        .source_control_write_grants()
        .authorize_browser_local(&auth_session_id, repo_id, 19)
        .expect_err("sync guard cleanup must revoke stale source control write grant");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_snapshot_request_rejects_missing_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session_without_sync_scope(&state, repo_id, 19)?;

    handle_sync_snapshot_request(&state, &ch, &mut session, PeerId::new("browser"), repo_id, None)
        .await;

    let (error, scope_nonce) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(scope_nonce, Some(19));
    assert_runtime_binding_cleared(&session);
    Ok(())
}

#[test]
fn browser_writer_registration_rejects_stale_scope_nonce_with_scoped_error() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session_without_sync_scope(&state, repo_id, 13)?;
    session.set_sync_scope_nonce(13);

    handle_register_writer(&state, &ch, &mut session, repo_id, PeerId::new("browser"), 11);

    let (error, scope_nonce) = try_recv_protocol_error(&mut rx);
    assert_eq!(error.code, ServerErrorCode::ScStaleScope);
    assert_eq!(scope_nonce, Some(11));
    assert_runtime_binding_cleared(&session);
    Ok(())
}

#[test]
fn browser_writer_registration_rejects_degraded_local_projection() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    state
        .sync_manager
        .mark_projection_writeback_fault("default");
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session_without_sync_scope(&state, repo_id, 37)?;
    let auth_session_id = AuthSessionId::for_test("degraded-writer-registration");
    let writer = PeerId::new("browser");
    session.bind_auth_session(auth_session_id.clone());
    session.set_sync_scope_nonce(37);
    state.source_control_write_grants().grant(
        auth_session_id.clone(),
        repo_id,
        SourceControlGrantBranch::Local,
        writer.clone(),
        37,
    );
    assert!(
        state
            .source_control_write_grants()
            .authorize_browser_local(&auth_session_id, repo_id, 37)
            .is_ok()
    );

    handle_register_writer(&state, &ch, &mut session, repo_id, writer, 37);

    let (error, scope_nonce) = try_recv_protocol_error(&mut rx);
    assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
    assert_eq!(scope_nonce, Some(37));
    assert!(session.writer_identity.is_none());
    state
        .source_control_write_grants()
        .authorize_browser_local(&auth_session_id, repo_id, 37)
        .expect_err("degraded writer registration must revoke source control write grant");
    Ok(())
}

#[test]
fn browser_writer_registration_rejects_broken_workspace_identity() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let workspace = state.repo.ensure_local_repo_workspace_identity("default")?;
    std::fs::write(
        deve_core::utils::notegit::repo_identity_path(&workspace),
        format!(
            "version = 1\nrepo_id = \"{}\"\nrepo_name = \"default\"\n",
            uuid::Uuid::new_v4()
        ),
    )?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session_without_sync_scope(&state, repo_id, 41)?;
    let auth_session_id = AuthSessionId::for_test("broken-workspace-identity");
    let writer = PeerId::new("browser");
    session.bind_auth_session(auth_session_id.clone());
    session.set_sync_scope_nonce(41);
    state.source_control_write_grants().grant(
        auth_session_id.clone(),
        repo_id,
        SourceControlGrantBranch::Local,
        writer.clone(),
        41,
    );

    handle_register_writer(&state, &ch, &mut session, repo_id, writer, 41);

    let (error, scope_nonce) = try_recv_protocol_error(&mut rx);
    assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
    assert_eq!(scope_nonce, Some(41));
    assert!(
        error
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("identity marker"))
    );
    assert!(session.writer_identity.is_none());
    state
        .source_control_write_grants()
        .authorize_browser_local(&auth_session_id, repo_id, 41)
        .expect_err("broken workspace identity must revoke source control write grant");
    Ok(())
}
