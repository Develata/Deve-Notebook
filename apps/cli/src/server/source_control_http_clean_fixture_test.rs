//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!
//! Source Control smoke fixture that never depends on checked-in dev ledger
//! state.

use super::support::{ProxyHarness, path_target, seed_pending, write_workspace_file};
use deve_core::source_control::{ChangeEntry, ChangeStatus, CommitInfo};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clean_source_control_smoke_fixture_stage_unstage_commit() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo_name = harness.repo.local_repo_name();
    assert!(
        harness
            .repo
            .list_pending_fs_in_local_repo(repo_name)?
            .is_empty()
    );
    assert!(harness.repo.list_staged_in_local_repo(repo_name)?.is_empty());

    write_workspace_file(&harness.dir, "smoke/a.md", "hello");
    seed_pending(
        &harness.repo,
        "smoke/a.md",
        ChangeStatus::Added,
        "hello",
    );
    let initial = http_status(&harness).await?;
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].path, "smoke/a.md");

    post_path(&harness, "/api/sc/stage-pending", "smoke/a.md").await?;
    assert!(
        harness
            .repo
            .list_pending_fs_in_local_repo(repo_name)?
            .is_empty()
    );
    assert_eq!(harness.repo.list_staged_in_local_repo(repo_name)?.len(), 1);

    post_path(&harness, "/api/sc/unstage", "smoke/a.md").await?;
    assert_eq!(
        harness
            .repo
            .list_pending_fs_in_local_repo(repo_name)?
            .len(),
        1
    );
    assert!(harness.repo.list_staged_in_local_repo(repo_name)?.is_empty());

    post_path(&harness, "/api/sc/stage-pending", "smoke/a.md").await?;
    let commit = post_commit(&harness, "clean smoke fixture").await?;
    assert_eq!(commit.doc_count, 1);
    assert_eq!(commit.message, "clean smoke fixture");
    assert!(
        harness
            .repo
            .list_pending_fs_in_local_repo(repo_name)?
            .is_empty()
    );
    assert!(harness.repo.list_staged_in_local_repo(repo_name)?.is_empty());

    harness.shutdown().await;
    Ok(())
}

async fn http_status(harness: &ProxyHarness) -> anyhow::Result<Vec<ChangeEntry>> {
    let response = harness
        .client
        .get(format!("{}/api/sc/status", harness.base_url))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    Ok(response.json().await?)
}

async fn post_path(harness: &ProxyHarness, endpoint: &str, path: &str) -> anyhow::Result<()> {
    let response = harness
        .client
        .post(format!("{}{}", harness.base_url, endpoint))
        .json(&path_target(path))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
    Ok(())
}

async fn post_commit(harness: &ProxyHarness, message: &str) -> anyhow::Result<CommitInfo> {
    let response = harness
        .client
        .post(format!("{}/api/sc/commit", harness.base_url))
        .json(&serde_json::json!({ "message": message }))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    Ok(response.json().await?)
}
