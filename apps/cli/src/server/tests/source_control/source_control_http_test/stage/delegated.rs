//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::super::super::auth::delegated_source_control::DELEGATED_SC_HEADER;
use super::super::support::{path_target, seed_pending, ProxyHarness};
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ServerErrorCode;
use deve_core::source_control::{ChangeStatus, SourceControlApi};

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
    assert_eq!(body.code, ServerErrorCode::PluginCapabilityDenied);
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
async fn delegated_source_control_rejects_unexpected_scope_nonce() -> anyhow::Result<()> {
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
        .header(
            DELEGATED_SC_HEADER,
            harness.delegated_source_control_header_value(),
        )
        .json(&serde_json::json!({
            "scope_nonce": 9,
            "path": "notes/a.md",
        }))
        .send()
        .await?;
    let status = response.status();
    let body: deve_core::protocol::ServerError = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(body.code, ServerErrorCode::ScStaleScope);
    assert_eq!(
        body.detail.as_deref(),
        Some("delegated source control scope nonce mismatch")
    );
    assert_eq!(repo.list_pending_fs_in_repo(&selector)?.len(), 1);
    assert!(repo.list_staged_in_repo(&selector)?.is_empty());
    harness.shutdown().await;
    Ok(())
}
