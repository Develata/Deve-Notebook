//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::super::support::{ProxyHarness, seed_pending};
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ServerErrorCode;
use deve_core::source_control::{ChangeStatus, SourceControlApi};
use reqwest::header::COOKIE as HTTP_COOKIE;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logout_revokes_source_control_write_grant() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    harness.grant_browser_write(1)?;

    let response = harness
        .client
        .post(format!("{}/api/auth/logout", harness.base_url))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);

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
    assert_eq!(body.code, ServerErrorCode::ScStaleScope);
    assert_eq!(repo.list_pending_fs_in_repo(&selector)?.len(), 1);
    assert!(repo.list_staged_in_repo(&selector)?.is_empty());
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
    assert_eq!(body.code, ServerErrorCode::ScStaleScope);
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
async fn http_source_control_jwt_grant_is_not_shadowed_by_dev_session_cookie() -> anyhow::Result<()>
{
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
