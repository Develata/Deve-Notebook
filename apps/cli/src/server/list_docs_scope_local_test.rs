//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::support::{build_state, seed_doc};
use crate::server::{channel::DualChannel, handlers::listing::handle_list_docs, session::WsSession};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_rejects_stale_local_selector() -> anyhow::Result<()> {
    let (_dir, state, _test_repo_id) = build_state()?;
    let _ = seed_doc(&state, "test", "notes/b.md", "from test")?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(13));
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    session.switch_repo("test".into(), Some(default_id));

    handle_list_docs(&state, &ch, &mut session, Some("req-1".into()), None).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, Some(13));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(
        (session.active_repo.as_deref(), session.active_repo_id),
        (None, None)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_on_scoped_local_unbound_state_returns_repo_unbound() -> anyhow::Result<()> {
    let (_dir, state, _test_repo_id) = build_state()?;
    let local_db = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.set_scope_nonce(Some(9));
    session.set_active_db(local_db);

    handle_list_docs(&state, &ch, &mut session, None, None).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::SyncRepoUnbound);
        }
        other => panic!("expected SyncRepoUnbound error, got {:?}", other),
    }
    assert!(session.get_active_db().is_none());
    Ok(())
}
