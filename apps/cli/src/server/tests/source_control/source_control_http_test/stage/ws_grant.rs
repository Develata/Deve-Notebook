//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::super::super::sync_hello_test_support::signed_hello_for_scope;
use super::super::super::ws_protocol_acceptance_support::{
    recv_server_message, send_client_message,
};
use super::super::support::{seed_pending, ProxyHarness};
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::security::IdentityKeyPair;
use deve_core::source_control::{ChangeStatus, SourceControlApi};
use reqwest::header::{COOKIE as HTTP_COOKIE, SET_COOKIE};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::COOKIE as WS_COOKIE;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type TestWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anonymous_localhost_source_control_write_grant_roundtrips_status_ws_and_http(
) -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();
    let scope_nonce = 1;
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");

    let cookie = fetch_dev_session_cookie(&harness).await?;
    let other_cookie = fetch_dev_session_cookie(&harness).await?;
    assert_ne!(cookie, other_cookie);

    let mut ws = connect_ws_with_cookie(&harness, &cookie).await?;
    register_writer_over_ws(&harness, &mut ws, scope_nonce).await?;

    let response = reqwest::Client::builder()
        .no_proxy()
        .build()?
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .header(HTTP_COOKIE, other_cookie)
        .json(&serde_json::json!({
            "scope_nonce": scope_nonce,
            "path": "notes/a.md",
        }))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(repo.list_pending_fs_in_repo(&selector)?.len(), 1);
    assert!(repo.list_staged_in_repo(&selector)?.is_empty());

    let response = reqwest::Client::builder()
        .no_proxy()
        .build()?
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .header(HTTP_COOKIE, cookie)
        .json(&serde_json::json!({
            "scope_nonce": scope_nonce,
            "path": "notes/a.md",
        }))
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
    assert!(repo.list_pending_fs_in_repo(&selector)?.is_empty());
    assert_eq!(repo.list_staged_in_repo(&selector)?.len(), 1);
    drop(ws);
    harness.shutdown().await;
    Ok(())
}

async fn fetch_dev_session_cookie(harness: &ProxyHarness) -> anyhow::Result<String> {
    let response = reqwest::Client::builder()
        .no_proxy()
        .build()?
        .get(format!("{}/api/auth/status", harness.base_url))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let set_cookie = response
        .headers()
        .get(SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| anyhow::anyhow!("missing dev session Set-Cookie"))?;
    let cookie = set_cookie
        .split(';')
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty dev session Set-Cookie"))?
        .to_string();
    assert!(cookie.starts_with("deve_dev_session="));
    Ok(cookie)
}

async fn connect_ws_with_cookie(harness: &ProxyHarness, cookie: &str) -> anyhow::Result<TestWs> {
    let ws_url = format!("{}/ws", harness.base_url.replacen("http://", "ws://", 1));
    let mut request = ws_url.into_client_request()?;
    request.headers_mut().insert(WS_COOKIE, cookie.parse()?);
    let (ws, _response) = connect_async(request).await?;
    Ok(ws)
}

async fn register_writer_over_ws(
    harness: &ProxyHarness,
    ws: &mut TestWs,
    scope_nonce: u64,
) -> anyhow::Result<()> {
    let repo_name = harness.repo.local_repo_name();
    let repo_id = harness
        .repo
        .get_repo_info_for(None, Some(repo_name))?
        .ok_or_else(|| anyhow::anyhow!("missing local repo info"))?
        .uuid;
    let remote = IdentityKeyPair::generate();
    send_client_message(
        ws,
        ClientMessage::SwitchRepoExact {
            repo_id,
            switch_nonce: Some(scope_nonce),
        },
    )
    .await?;
    for _ in 0..3 {
        let _ = recv_server_message(ws).await?;
    }

    let hello = signed_hello_for_scope(&remote, repo_id, scope_nonce);
    send_client_message(
        ws,
        ClientMessage::SyncHello {
            peer_id: hello.peer_id,
            peer_pubkey: hello.peer_pubkey,
            session_proof: hello.session_proof,
            vector: hello.remote_vector,
            repo_id: hello.repo_id,
            scope_nonce: hello.scope_nonce.into(),
        },
    )
    .await?;
    let _ = recv_server_message(ws).await?;
    let _ = recv_server_message(ws).await?;

    send_client_message(
        ws,
        ClientMessage::RegisterWriter {
            peer_id: remote.peer_id(),
            repo_id,
            scope_nonce: scope_nonce.into(),
        },
    )
    .await?;
    match recv_server_message(ws).await? {
        ServerMessage::WriteReady {
            peer_id,
            repo_id: actual_repo_id,
            scope_nonce: actual_scope_nonce,
            branch,
        } => {
            assert_eq!(peer_id, remote.peer_id());
            assert_eq!(actual_repo_id, repo_id);
            assert_eq!(actual_scope_nonce.get(), scope_nonce);
            assert_eq!(branch, None);
        }
        other => panic!("expected WriteReady, got {other:?}"),
    }
    Ok(())
}
