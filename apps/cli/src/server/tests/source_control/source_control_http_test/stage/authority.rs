//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::super::super::source_control_grants::SourceControlGrantBranch;
use super::super::support::{path_target, seed_pending, ProxyHarness};
use crate::server::runtime::watcher_runtime::{RepoMountState, WatcherRuntimeView};
use deve_core::ledger::traits::RepoSelector;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerError, ServerErrorCode};
use deve_core::source_control::{ChangeStatus, SourceControlApi};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_source_control_all_mutations_require_browser_write_grant() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();

    seed_pending(&repo, "notes/stage.md", ChangeStatus::Added, "stage");
    seed_pending(&repo, "notes/discard.md", ChangeStatus::Added, "discard");
    seed_pending(&repo, "notes/unstage.md", ChangeStatus::Added, "unstage");
    seed_pending(&repo, "notes/commit.md", ChangeStatus::Added, "commit");
    repo.stage_pending_in_repo(&selector, &path_target("notes/unstage.md"))?;
    repo.stage_pending_in_repo(&selector, &path_target("notes/commit.md"))?;

    assert_stale_grant(
        harness
            .client
            .post(format!("{}/api/sc/stage-pending", harness.base_url))
            .json(&serde_json::json!({
                "scope_nonce": 9,
                "path": "notes/stage.md",
            }))
            .send()
            .await?,
    )
    .await?;
    assert_stale_grant(
        harness
            .client
            .post(format!("{}/api/sc/discard-pending", harness.base_url))
            .json(&serde_json::json!({
                "scope_nonce": 9,
                "path": "notes/discard.md",
            }))
            .send()
            .await?,
    )
    .await?;
    assert_stale_grant(
        harness
            .client
            .post(format!("{}/api/sc/unstage", harness.base_url))
            .json(&serde_json::json!({
                "scope_nonce": 9,
                "path": "notes/unstage.md",
            }))
            .send()
            .await?,
    )
    .await?;
    assert_stale_grant(
        harness
            .client
            .post(format!("{}/api/sc/commit", harness.base_url))
            .json(&serde_json::json!({
                "scope_nonce": 9,
                "message": "must not commit without grant",
            }))
            .send()
            .await?,
    )
    .await?;

    let pending = repo.list_pending_fs_in_repo(&selector)?;
    assert!(pending.iter().any(|entry| entry.path == "notes/stage.md"));
    assert!(pending.iter().any(|entry| entry.path == "notes/discard.md"));
    let staged = repo.list_staged_in_repo(&selector)?;
    assert!(staged.iter().any(|entry| entry.path == "notes/unstage.md"));
    assert!(staged.iter().any(|entry| entry.path == "notes/commit.md"));

    harness.shutdown().await;
    Ok(())
}

async fn assert_stale_grant(response: reqwest::Response) -> anyhow::Result<()> {
    let status = response.status();
    let body: ServerError = response.json().await?;
    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(body.code, ServerErrorCode::ScStaleScope);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_ingestion_error_mapping_returns_json_503() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo_id = harness
        .repo
        .get_repo_info()?
        .expect("default repo metadata")
        .uuid;
    harness.grant_browser_write(1)?;
    harness
        .state
        .set_watcher_runtime_view_for_test(WatcherRuntimeView::with_state_for_test(
            repo_id,
            1,
            RepoMountState::Failed,
        ));
    seed_pending(
        &harness.repo,
        "notes/unavailable.md",
        ChangeStatus::Added,
        "pending",
    );

    let response = harness
        .client
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .json(&serde_json::json!({
            "scope_nonce": 1,
            "path": "notes/unavailable.md",
        }))
        .send()
        .await?;
    let status = response.status();
    let body: ServerError = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body.code,
        ServerErrorCode::StorageWorkspaceIngestionUnavailable
    );
    assert_eq!(
        body.detail.as_deref(),
        Some("Workspace changes are temporarily unavailable; restart the service to recover.")
    );
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
    assert_eq!(body.code, ServerErrorCode::ScRepoContextInvalid);
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
    assert_eq!(body.code, ServerErrorCode::ScStaleScope);
    assert_eq!(repo.list_pending_fs_in_repo(&selector)?.len(), 1);
    assert!(repo.list_staged_in_repo(&selector)?.is_empty());

    repo.stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    repo.unstage_file_in_repo(&selector, &path_target("notes/a.md"))?;
    assert_eq!(repo.list_pending_fs_in_repo(&selector)?.len(), 1);
    harness.shutdown().await;
    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_source_control_write_grant_requires_local_branch() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo = harness.repo.clone();
    let selector = RepoSelector::default();
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    let repo_id = repo
        .get_repo_info_for(None, Some(repo.local_repo_name()))?
        .ok_or_else(|| anyhow::anyhow!("missing local repo info"))?
        .uuid;
    harness
        .state
        .source_control_write_grants()
        .grant(
            harness.auth_session_id.clone(),
            repo_id,
            SourceControlGrantBranch::Remote(PeerId::new("remote-peer")),
            PeerId::new("test-peer"),
            1,
        )
        .expect("source-control write grant");

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
