//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::{handle_switch_branch, handle_switch_repo};
use crate::server::{
    session::WsSession,
    switcher_test_support::{browser_session, build_state, unicast_channel},
};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

async fn assert_context_error(
    rx: &mut mpsc::Receiver<ServerMessage>,
    expected_scope_nonce: Option<u64>,
    expected_switch_nonce: Option<u64>,
    detail_contains: Option<&str>,
) -> anyhow::Result<()> {
    match rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            scope_nonce,
            switch_nonce,
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(scope_nonce, expected_scope_nonce);
            assert_eq!(switch_nonce, expected_switch_nonce);
            if let Some(detail) = detail_contains {
                assert!(
                    error
                        .detail
                        .as_deref()
                        .is_some_and(|actual| actual.contains(detail))
                );
            }
            Ok(())
        }
        other => anyhow::bail!("expected ProtocolError, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_requires_switch_nonce_for_browser_sessions() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(7);

    handle_switch_branch(&state, &ch, &mut session, None, None).await;
    assert_context_error(&mut uni_rx, Some(7), None, None).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_repo_requires_switch_nonce_for_browser_sessions() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(11);

    handle_switch_repo(&state, &ch, &mut session, "default".into(), None, None).await;
    assert_context_error(&mut uni_rx, Some(11), None, None).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_rejects_stale_switch_nonce_for_browser_sessions() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(7);

    handle_switch_branch(&state, &ch, &mut session, None, Some(7)).await;
    assert_context_error(&mut uni_rx, Some(7), Some(7), Some("stale")).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_repo_rejects_stale_switch_nonce_for_browser_sessions() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(11);

    handle_switch_repo(&state, &ch, &mut session, "default".into(), None, Some(10)).await;
    assert_context_error(&mut uni_rx, Some(11), Some(10), Some("stale")).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_rejects_non_browser_sessions() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = WsSession::new();

    handle_switch_branch(&state, &ch, &mut session, Some("peer-a".into()), None).await;

    assert_context_error(&mut uni_rx, None, None, Some("browser sessions")).await?;
    assert!(session.active_branch.is_none());
    assert!(session.active_repo.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_repo_rejects_non_browser_sessions() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = WsSession::new();

    handle_switch_repo(&state, &ch, &mut session, "default".into(), None, None).await;

    assert_context_error(&mut uni_rx, None, None, Some("browser sessions")).await?;
    assert!(session.active_branch.is_none());
    assert!(session.active_repo.is_none());
    Ok(())
}
