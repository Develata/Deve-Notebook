//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 07_diff_logic#merge-contract
//!
//! Merge route readonly gate tests.

use super::route_merge;
use crate::server::session::WsSession;
use crate::server::sync_hello_test_support::{build_state, unicast_channel};
use deve_core::ledger::RepoInfo;
use deve_core::models::PeerId;
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_manual_write_readonly_gate_rejects_remote_scope() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let peer_id = ensure_remote_repo(&state, repo_id)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_remote_session(&peer_id, repo_id, 37);

    for msg in manual_merge_write_messages(Some(37)) {
        route_merge(&state, &ch, &mut session, msg).await;
        expect_remote_readonly_error(&mut uni_rx, 37).await?;
    }
    Ok(())
}

fn ensure_remote_repo(
    state: &std::sync::Arc<crate::server::AppState>,
    repo_id: uuid::Uuid,
) -> anyhow::Result<PeerId> {
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "notes".into(),
            url: Some("urn:test:notes".into()),
        },
    )?;
    Ok(peer_id)
}

fn browser_remote_session(peer_id: &PeerId, repo_id: uuid::Uuid, scope_nonce: u64) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(scope_nonce));
    session
}

fn manual_merge_write_messages(scope_nonce: Option<u64>) -> Vec<ClientMessage> {
    vec![
        ClientMessage::SetSyncMode {
            mode: "manual".into(),
            scope_nonce,
        },
        ClientMessage::ConfirmMerge { scope_nonce },
        ClientMessage::DiscardPending { scope_nonce },
    ]
}

async fn expect_remote_readonly_error(
    uni_rx: &mut mpsc::Receiver<ServerMessage>,
    scope_nonce: u64,
) -> anyhow::Result<()> {
    match timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("protocol error")
    {
        ServerMessage::ProtocolError {
            error,
            scope_nonce: actual_scope_nonce,
            ..
        } => {
            assert_eq!(error.code, ServerErrorCode::ScRemoteBranchReadonly);
            assert_eq!(actual_scope_nonce, Some(scope_nonce));
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
    Ok(())
}
