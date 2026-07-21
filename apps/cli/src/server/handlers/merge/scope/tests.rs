//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Merge scope bootstrap tests.

use super::test_support::{app_state, build_state, init_repo, test_channel};
use super::{resolve_read_repo_id, resolve_write_repo_id};
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

#[test]
fn read_repo_id_uses_active_local_repo_without_sync_binding() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let projection_base = dir.path().join("notes");
    let repo = init_repo(&dir, &projection_base, Some("urn:default"))?;
    let test_id = crate::server::catalog_repo_support::catalog_additional_repo(
        &repo,
        &dir.path().join("ledger"),
        "test",
        &projection_base,
        10,
        None,
    )?;
    let state = app_state(Arc::new(repo))?;
    let ch = test_channel();
    let mut session = WsSession::new();
    session.switch_repo(test_id.to_string(), Some(test_id));

    assert_eq!(
        resolve_read_repo_id(&state, &ch, &mut session, None),
        Some(test_id)
    );
    assert_eq!(session.active_repo_id, Some(test_id));
    Ok(())
}

#[test]
fn read_repo_id_bootstraps_single_local_repo() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    let ch = test_channel();
    let mut session = WsSession::new();

    assert_eq!(
        resolve_read_repo_id(&state, &ch, &mut session, None),
        Some(default_id)
    );
    assert_eq!(
        session.active_repo.as_deref(),
        Some(default_id.to_string().as_str())
    );
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[test]
fn write_repo_id_uses_writer_ready_local_repo() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    state
        .repo
        .ensure_local_repo_workspace_identity(&default_id.to_string())?;
    let ch = test_channel();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(default_id.to_string(), Some(default_id));
    session.set_scope_nonce(Some(0));
    session.set_writer_identity(default_id, PeerId::new("browser-peer"), 0);

    assert_eq!(
        resolve_write_repo_id(&state, &ch, &mut session, Some(0)),
        Some(default_id)
    );
    assert_eq!(
        session.active_repo.as_deref(),
        Some(default_id.to_string().as_str())
    );
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[test]
fn write_repo_id_rejects_missing_writer_ready_scope() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    state
        .repo
        .ensure_local_repo_workspace_identity(&default_id.to_string())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(4);
    let ch = DualChannel::new(broadcast::channel(4).0, uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(default_id.to_string(), Some(default_id));
    session.set_scope_nonce(Some(41));

    assert_eq!(
        resolve_write_repo_id(&state, &ch, &mut session, Some(41)),
        None
    );
    match uni_rx.try_recv() {
        Ok(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
            assert_eq!(scope_nonce, Some(41));
        }
        other => panic!("expected writer-ready ProtocolError, got {other:?}"),
    }
    Ok(())
}

#[test]
fn write_repo_id_rejects_degraded_local_projection() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    state
        .sync_manager
        .mark_projection_writeback_fault(state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(4);
    let ch = DualChannel::new(broadcast::channel(4).0, uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(default_id.to_string(), Some(default_id));
    session.set_scope_nonce(Some(47));

    assert_eq!(
        resolve_write_repo_id(&state, &ch, &mut session, Some(47)),
        None
    );
    match uni_rx.try_recv() {
        Ok(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(scope_nonce, Some(47));
        }
        other => panic!("expected degraded projection ProtocolError, got {other:?}"),
    }
    Ok(())
}
