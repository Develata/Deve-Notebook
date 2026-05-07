use crate::server::session::WsSession;
use crate::server::sync_hello_test_support::{build_state, unicast_channel};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use tokio::time::{Duration, timeout};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_scope_gate_falls_through_from_docs_to_downstream_domains() -> anyhow::Result<()> {
    assert_route_stale_detail_via_docs(confirm_merge(16), "merge control scope nonce is stale")
        .await?;
    assert_route_stale_detail_via_docs(get_changes(16), "source control scope nonce is stale")
        .await?;
    assert_route_stale_detail_via_docs(search(16), "search scope nonce is stale").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_scope_gate_falls_through_from_merge_to_downstream_domains() -> anyhow::Result<()> {
    assert_route_stale_detail_via_merge(get_changes(16), "source control scope nonce is stale")
        .await?;
    assert_route_stale_detail_via_merge(search(16), "search scope nonce is stale").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_scope_gate_falls_through_from_source_control_to_core() -> anyhow::Result<()> {
    assert_route_stale_detail_via_source_control(search(16), "search scope nonce is stale").await
}

async fn assert_route_stale_detail_via_docs(
    msg: ClientMessage,
    expected_detail: &str,
) -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(17);

    super::docs::route_docs(&state, &ch, &mut session, msg).await;
    assert_stale_detail(
        recv_unicast_message(&mut uni_rx).await?,
        Some(16),
        expected_detail,
    );
    Ok(())
}

async fn assert_route_stale_detail_via_merge(
    msg: ClientMessage,
    expected_detail: &str,
) -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(17);

    super::merge::route_merge(&state, &ch, &mut session, msg).await;
    assert_stale_detail(
        recv_unicast_message(&mut uni_rx).await?,
        Some(16),
        expected_detail,
    );
    Ok(())
}

async fn assert_route_stale_detail_via_source_control(
    msg: ClientMessage,
    expected_detail: &str,
) -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(17);

    super::source_control::route_source_control(&state, &ch, &mut session, msg).await;
    assert_stale_detail(
        recv_unicast_message(&mut uni_rx).await?,
        Some(16),
        expected_detail,
    );
    Ok(())
}

fn browser_session(scope_nonce: u64) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

fn confirm_merge(scope_nonce: u64) -> ClientMessage {
    ClientMessage::ConfirmMerge {
        scope_nonce: Some(scope_nonce),
    }
}

fn get_changes(scope_nonce: u64) -> ClientMessage {
    ClientMessage::GetChanges {
        request_id: "changes".into(),
        scope_nonce: Some(scope_nonce),
    }
}

fn search(scope_nonce: u64) -> ClientMessage {
    ClientMessage::Search {
        request_id: "search".into(),
        query: "needle".into(),
        limit: 10,
        scope_nonce: Some(scope_nonce),
    }
}

async fn recv_unicast_message(
    uni_rx: &mut tokio::sync::mpsc::Receiver<ServerMessage>,
) -> anyhow::Result<ServerMessage> {
    Ok(timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("protocol error"))
}

fn assert_stale_detail(message: ServerMessage, scope_nonce: Option<u64>, expected_detail: &str) {
    match message {
        ServerMessage::ProtocolError {
            error,
            scope_nonce: actual_scope_nonce,
            ..
        } => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(actual_scope_nonce, scope_nonce);
            assert!(
                error
                    .detail
                    .as_deref()
                    .expect("detail")
                    .contains(expected_detail)
            );
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
}
