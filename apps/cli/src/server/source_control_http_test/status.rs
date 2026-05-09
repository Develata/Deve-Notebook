//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use super::support::ProxyHarness;
use deve_core::git_bridge::{
    GitMirrorRepairReview, init_table, mark_out_of_sync, queue_deve_commit,
};
use deve_core::ledger::RepoManager;
use deve_core::protocol::ServerErrorCode;
use deve_core::source_control::CommitInfo;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_http_status_requires_repo_selector_when_multiple_local_repos_exist()
-> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    RepoManager::init(harness.dir.path(), 10, Some("test"), Some("urn:test"))?;

    let response = harness
        .client
        .get(format!("{}/api/sc/status", harness.base_url))
        .send()
        .await?;
    let status = response.status();
    let body: deve_core::protocol::ServerError = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(body.code, ServerErrorCode::ScRepoNotSelected);
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_http_status_rejects_selector_mismatch() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let default_id = harness
        .repo
        .get_repo_info()?
        .expect("default repo info")
        .uuid;
    RepoManager::init(harness.dir.path(), 10, Some("test"), Some("urn:test"))?;

    let response = harness
        .client
        .get(format!("{}/api/sc/status", harness.base_url))
        .query(&[
            ("repo_id", default_id.to_string()),
            ("repo_name", "test".to_string()),
        ])
        .send()
        .await?;
    let status = response.status();
    let body: deve_core::protocol::ServerError = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::CONFLICT);
    assert_eq!(body.code, ServerErrorCode::ScRepoContextInvalid);
    assert!(
        body.detail
            .as_deref()
            .is_some_and(|detail| detail.contains("Repo selector mismatch"))
    );
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_http_status_maps_missing_repo_to_not_found() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;

    let response = harness
        .client
        .get(format!("{}/api/sc/status", harness.base_url))
        .query(&[("repo_name", "missing")])
        .send()
        .await?;
    let status = response.status();
    let body: deve_core::protocol::ServerError = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::NOT_FOUND);
    assert_eq!(body.code, ServerErrorCode::StorageNotFound);
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_git_mirror_repair_review_is_readonly_record_source() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo_id = harness
        .repo
        .get_repo_info()?
        .expect("default repo info")
        .uuid;
    harness
        .repo
        .run_on_local_repo(harness.repo.local_repo_name(), |db| {
            init_table(db)?;
            queue_deve_commit(db, repo_id, &commit("deve-1", 1))?;
            mark_out_of_sync(
                db,
                "deve-1",
                "Git mirror refuses to include path(s) outside queued Deve commit: docs/example.md",
            )?;
            Ok(())
        })?;

    let response = harness
        .client
        .get(format!(
            "{}/api/sc/git-mirror/repair-review",
            harness.base_url
        ))
        .send()
        .await?;
    let status = response.status();
    let body: GitMirrorRepairReview = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::OK);
    assert!(body.manual_only);
    assert_eq!(body.records.len(), 1);
    assert_eq!(body.records[0].action_code, "resolve_projection_scope");
    assert_eq!(body.records[0].subject, "docs/example.md");
    assert_eq!(
        body.records[0].retry_command.as_deref(),
        Some("deve_cli git export --repo default --retry-out-of-sync")
    );
    harness.shutdown().await;
    Ok(())
}

fn commit(id: &str, ledger_seq: u64) -> CommitInfo {
    CommitInfo {
        id: id.to_string(),
        parent_id: None,
        message: "commit".to_string(),
        timestamp: 1,
        doc_count: 1,
        ledger_seq,
    }
}
