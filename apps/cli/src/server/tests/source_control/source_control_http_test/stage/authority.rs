//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::super::support::{ProxyHarness, path_target, seed_pending};
use deve_core::ledger::traits::RepoSelector;
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
    assert!(
        pending
            .iter()
            .any(|entry| entry.path == "notes/discard.md")
    );
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
