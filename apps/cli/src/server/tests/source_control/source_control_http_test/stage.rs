//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::{
    ProxyHarness, default_workspace_root, path_target, seed_pending, seed_tracked_rename,
    write_workspace_file,
};
use super::super::sync_hello_test_support::signed_hello_for_scope;
use super::super::ws_protocol_acceptance_support::{recv_server_message, send_client_message};
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::{ClientMessage, ScPathTarget, ServerMessage};
use deve_core::security::IdentityKeyPair;
use deve_core::source_control::{ChangeStatus, SourceControlApi};
use reqwest::header::{COOKIE as HTTP_COOKIE, SET_COOKIE};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::COOKIE as WS_COOKIE;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type TestWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_unstage_roundtrip() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let proxy = harness.proxy.clone();
    let selector = RepoSelector::default();
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    proxy.stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    assert!(proxy.list_pending_fs_in_repo(&selector)?.is_empty());
    let staged =
        deve_core::source_control::SourceControlApi::list_staged_in_repo(&proxy, &selector)?;
    assert_eq!(staged.len(), 1);
    assert_eq!(staged[0].path, "notes/a.md");
    proxy.unstage_file_in_repo(&selector, &path_target("notes/a.md"))?;
    let pending = proxy.list_pending_fs_in_repo(&selector)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/a.md");
    assert_eq!(pending[0].status, ChangeStatus::Added);
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_rename_candidate_collapses_and_stages_pair() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let dir = &harness.dir;
    let repo = harness.repo.clone();
    let proxy = harness.proxy.clone();
    let selector = RepoSelector::default();
    write_workspace_file(dir, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    proxy.stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    proxy.commit_staged_in_repo(&selector, "initial")?;
    let doc_id = repo.get_docid("notes/a.md")?.expect("existing doc id");

    write_workspace_file(dir, "notes/b.md", "hello");
    std::fs::remove_file(default_workspace_root(dir).join("notes/a.md"))?;
    seed_tracked_rename(&repo, doc_id, "notes/a.md", "notes/b.md", "hello");

    let pending = proxy.list_pending_fs_in_repo(&selector)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/b.md");
    assert_eq!(pending[0].renamed_from.as_deref(), Some("notes/a.md"));

    proxy.stage_pending_in_repo(
        &selector,
        &ScPathTarget {
            path: "notes/b.md".into(),
            doc_id: Some(doc_id),
        },
    )?;
    assert!(proxy.list_pending_fs_in_repo(&selector)?.is_empty());
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_http_stage_prefers_doc_id_over_stale_path() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let dir = &harness.dir;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();
    write_workspace_file(dir, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    repo.stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    repo.commit_staged_in_repo(&selector, "initial")?;
    let doc_id = repo.get_docid("notes/a.md")?.expect("existing doc id");

    write_workspace_file(dir, "notes/b.md", "world");
    std::fs::remove_file(default_workspace_root(dir).join("notes/a.md"))?;
    seed_tracked_rename(&repo, doc_id, "notes/a.md", "notes/b.md", "world");
    harness.grant_browser_write(1)?;

    let response = harness
        .client
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .json(&serde_json::json!({
            "scope_nonce": 1,
            "path": "notes/a.md",
            "doc_id": doc_id.to_string(),
        }))
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
    assert!(repo.list_pending_fs_in_repo(&selector)?.is_empty());
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_http_stage_rejects_missing_scope_nonce_before_mutation() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");

    let response = harness
        .client
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .json(&serde_json::json!({
            "path": "notes/a.md",
        }))
        .send()
        .await?;
    let status = response.status();
    let body: deve_core::protocol::ServerError = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(body.code, deve_core::protocol::ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(
        body.detail.as_deref(),
        Some("source control scope nonce missing")
    );
    assert_eq!(repo.list_pending_fs_in_repo(&selector)?.len(), 1);
    assert!(repo.list_staged_in_repo(&selector)?.is_empty());
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_source_control_mutations_require_browser_write_grant() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");

    let response = harness
        .client
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .json(&serde_json::json!({
            "scope_nonce": 1,
            "path": "notes/a.md",
        }))
        .send()
        .await?;
    let status = response.status();
    let body: deve_core::protocol::ServerError = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(
        body.code,
        deve_core::protocol::ServerErrorCode::ScStaleScope
    );
    assert_eq!(repo.list_pending_fs_in_repo(&selector)?.len(), 1);
    assert!(repo.list_staged_in_repo(&selector)?.is_empty());

    repo.stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    repo.unstage_file_in_repo(&selector, &path_target("notes/a.md"))?;
    assert_eq!(repo.list_pending_fs_in_repo(&selector)?.len(), 1);
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delegated_remote_proxy_scope_nonce_is_not_main_http_grant() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");

    let response = harness
        .client
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .json(&serde_json::json!({
            "scope_nonce": 1,
            "path": "notes/a.md",
        }))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);

    harness
        .proxy
        .stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    assert!(repo.list_pending_fs_in_repo(&selector)?.is_empty());
    assert_eq!(repo.list_staged_in_repo(&selector)?.len(), 1);
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delegated_source_control_requires_proxy_capability() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");

    let response = harness
        .client
        .post(format!(
            "{}/api/delegated/sc/stage-pending",
            harness.base_url
        ))
        .json(&serde_json::json!({
            "scope_nonce": 1,
            "path": "notes/a.md",
        }))
        .send()
        .await?;
    let status = response.status();
    let body: deve_core::protocol::ServerError = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        body.code,
        deve_core::protocol::ServerErrorCode::PluginCapabilityDenied
    );
    assert_eq!(repo.list_pending_fs_in_repo(&selector)?.len(), 1);
    assert!(repo.list_staged_in_repo(&selector)?.is_empty());

    harness
        .proxy
        .stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    assert!(repo.list_pending_fs_in_repo(&selector)?.is_empty());
    assert_eq!(repo.list_staged_in_repo(&selector)?.len(), 1);
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anonymous_localhost_source_control_grant_is_not_dev_wide() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    harness.grant_browser_write(1)?;

    let response = harness
        .client
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .header(
            reqwest::header::COOKIE,
            harness.dev_session_cookie_header_for("other-browser-session"),
        )
        .json(&serde_json::json!({
            "scope_nonce": 1,
            "path": "notes/a.md",
        }))
        .send()
        .await?;
    let status = response.status();
    let body: deve_core::protocol::ServerError = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(
        body.code,
        deve_core::protocol::ServerErrorCode::ScStaleScope
    );
    assert_eq!(repo.list_pending_fs_in_repo(&selector)?.len(), 1);
    assert!(repo.list_staged_in_repo(&selector)?.is_empty());

    let response = harness
        .client
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .json(&serde_json::json!({
            "scope_nonce": 1,
            "path": "notes/a.md",
        }))
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
    assert!(repo.list_pending_fs_in_repo(&selector)?.is_empty());
    assert_eq!(repo.list_staged_in_repo(&selector)?.len(), 1);
    harness.shutdown().await;
    Ok(())
}

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_source_control_jwt_grant_is_not_shadowed_by_dev_session_cookie(
) -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();
    let scope_nonce = 1;
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");

    let (jwt_cookie, jwt_auth_session_id) = harness.jwt_cookie_header_and_auth_session()?;
    harness.grant_browser_write_for_auth_session(jwt_auth_session_id, scope_nonce)?;
    let dev_cookie = harness.dev_session_cookie_header_for("same-browser-dev-cookie");
    let combined_cookie = format!("{jwt_cookie}; {dev_cookie}");

    let response = reqwest::Client::builder()
        .no_proxy()
        .build()?
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .header(HTTP_COOKIE, combined_cookie)
        .json(&serde_json::json!({
            "scope_nonce": scope_nonce,
            "path": "notes/a.md",
        }))
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
    assert!(repo.list_pending_fs_in_repo(&selector)?.is_empty());
    assert_eq!(repo.list_staged_in_repo(&selector)?.len(), 1);
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
            name: repo_name.to_string(),
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
