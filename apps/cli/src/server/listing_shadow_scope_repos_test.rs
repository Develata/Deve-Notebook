//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::support::{bind_stale_runtime, build_state};
use crate::server::{
    channel::DualChannel, handlers::listing::handle_list_repos, session::WsSession,
};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_repos_on_unbound_remote_with_stale_runtime_binding_clears_scope() -> anyhow::Result<()>
{
    let (_dir, state) = build_state()?;
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(55));
    session.switch_branch(Some("valid-peer".into()));
    bind_stale_runtime(&state, &mut session, default_id, 55)?;

    handle_list_repos(&state, &ch, &mut session, Some("req-repos".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
        }
        other => panic!(
            "expected ProtocolError for stale runtime binding, got {:?}",
            other
        ),
    }
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
