//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Merge scope bootstrap tests.

use super::test_support::{app_state, build_state, init_repo, test_channel};
use super::{resolve_read_repo_id, resolve_write_repo_id};
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::{broadcast, mpsc};

#[test]
fn read_repo_id_uses_active_local_repo_without_sync_binding() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let projection_base = dir.path().join("notes");
    let repo = init_repo(&dir, &projection_base, "default", Some("urn:default"))?;
    let test_repo = init_repo(&dir, &projection_base, "test", None)?;
    let test_id = test_repo.get_repo_info()?.expect("test info").uuid;
    let state = app_state(Arc::new(repo))?;
    let ch = test_channel();
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(test_id));

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
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[test]
fn write_repo_id_bootstraps_single_local_repo() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    state.repo.ensure_local_repo_workspace_identity("default")?;
    let ch = test_channel();
    let mut session = WsSession::new();

    assert_eq!(
        resolve_write_repo_id(&state, &ch, &mut session, None),
        Some(default_id)
    );
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[test]
fn write_repo_id_rejects_degraded_local_projection() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    state
        .sync_manager
        .mark_projection_writeback_fault("default");
    let (uni_tx, mut uni_rx) = mpsc::channel(4);
    let ch = DualChannel::new(broadcast::channel(4).0, uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo("default".into(), Some(default_id));
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
