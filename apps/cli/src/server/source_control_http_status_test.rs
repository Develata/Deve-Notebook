//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use super::support::ProxyHarness;
use deve_core::ledger::RepoManager;
use deve_core::protocol::ServerErrorCode;

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
