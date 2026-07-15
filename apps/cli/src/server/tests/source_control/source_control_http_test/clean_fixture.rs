//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
//! Source Control smoke fixture that never depends on checked-in dev ledger
//! state.

use super::support::{ProxyHarness, seed_pending, write_workspace_file};
use deve_core::git_bridge::get_record;
use deve_core::protocol::{
    DocumentRecoveryScope, ProjectionRecoveryCause, ServerError, ServerErrorCode, ServerMessage,
};
use deve_core::source_control::{
    ChangeEntry, ChangeStatus, CommitInfo, ExternalApplyReceipt,
};

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
    assert!(
        harness
            .repo
            .list_staged_in_local_repo(repo_name)?
            .is_empty()
    );

    write_workspace_file(&harness.dir, "smoke/a.md", "hello");
    seed_pending(&harness.repo, "smoke/a.md", ChangeStatus::Added, "hello");
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
        harness.repo.list_pending_fs_in_local_repo(repo_name)?.len(),
        1
    );
    assert!(
        harness
            .repo
            .list_staged_in_local_repo(repo_name)?
            .is_empty()
    );

    post_path(&harness, "/api/sc/stage-pending", "smoke/a.md").await?;
    let applied = post_apply_external_changes(&harness).await?;
    assert_eq!(applied.applied_target_count, 1);
    assert_eq!(applied.affected_docs.len(), 1);
    assert!(applied.authority_head.storage_key() > 0);
    assert!(
        harness
            .repo
            .list_staged_in_local_repo(repo_name)?
            .is_empty()
    );
    assert_eq!(
        harness
            .repo
            .list_confirmed_ledger_changes_in_local_repo(repo_name)?
            .len(),
        1
    );
    let commit = post_commit(&harness, "clean smoke fixture").await?;
    assert_eq!(commit.doc_count, 1);
    assert_eq!(commit.message, "clean smoke fixture");
    assert!(
        harness
            .repo
            .list_confirmed_ledger_changes_in_local_repo(repo_name)?
            .is_empty()
    );
    assert!(http_status(&harness).await?.is_empty());
    assert!(
        harness
            .repo
            .list_pending_fs_in_local_repo(repo_name)?
            .is_empty()
    );
    assert!(
        harness
            .repo
            .list_staged_in_local_repo(repo_name)?
            .is_empty()
    );

    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_external_apply_returns_receipt_and_broadcasts_one_recovery() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    write_workspace_file(&harness.dir, "smoke/live.md", "hello");
    seed_pending(&harness.repo, "smoke/live.md", ChangeStatus::Added, "hello");
    post_path(&harness, "/api/sc/stage-pending", "smoke/live.md").await?;
    let mut events = harness.state.tx.subscribe();

    let applied = post_apply_external_changes(&harness).await?;
    assert_eq!(applied.applied_target_count, 1);
    let recovery = tokio::time::timeout(std::time::Duration::from_secs(1), events.recv()).await??;
    match recovery {
        ServerMessage::ProjectionRecoveryRequired(required) => {
            assert_eq!(required.repo_id, applied.repo_id);
            assert_eq!(required.cause, ProjectionRecoveryCause::ExternalApply);
            assert_eq!(
                required.plan.documents,
                DocumentRecoveryScope::Exact(applied.affected_docs.clone())
            );
            assert!(required.plan.refresh_doc_list);
            assert!(required.plan.refresh_source_control);
            assert!(required.plan.refresh_external_changes);
        }
        other => panic!("expected one typed recovery after External Apply, got {other:?}"),
    }
    assert!(
        events.try_recv().is_err(),
        "apply published unexpected extra events"
    );

    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_source_control_write_rejects_degraded_local_projection() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo_name = harness.repo.local_repo_name();
    write_workspace_file(&harness.dir, "smoke/degraded.md", "hello");
    seed_pending(
        &harness.repo,
        "smoke/degraded.md",
        ChangeStatus::Added,
        "hello",
    );
    harness
        .sync_manager
        .mark_projection_writeback_fault(repo_name);
    harness.grant_browser_write(1)?;

    let response = harness
        .client
        .post(format!("{}/api/sc/stage-pending", harness.base_url))
        .json(&serde_json::json!({
            "scope_nonce": 1,
            "path": "smoke/degraded.md",
        }))
        .send()
        .await?;
    let status = response.status();
    let body: ServerError = response.json().await?;

    assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body.code, ServerErrorCode::StoragePersistFailed);
    assert_eq!(
        harness.repo.list_pending_fs_in_local_repo(repo_name)?.len(),
        1
    );
    assert!(
        harness
            .repo
            .list_staged_in_local_repo(repo_name)?
            .is_empty()
    );
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_source_control_commit_always_queues_git_main_mirror() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let repo_name = harness.repo.local_repo_name();
    let repo_root = harness.repo.local_repo_workspace_root(repo_name)?;
    std::fs::create_dir_all(repo_root.join(".git"))?;
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&repo_root)?;

    write_workspace_file(&harness.dir, "smoke/mirror.md", "hello");
    seed_pending(
        &harness.repo,
        "smoke/mirror.md",
        ChangeStatus::Added,
        "hello",
    );
    post_path(&harness, "/api/sc/stage-pending", "smoke/mirror.md").await?;
    post_apply_external_changes(&harness).await?;

    let commit = post_commit(&harness, "ngit mirror").await?;
    let record = harness
        .repo
        .run_on_local_repo(repo_name, |db| Ok(get_record(db, &commit.id)?))?;

    assert!(record.is_some());
    assert!(
        harness
            .repo
            .list_staged_in_local_repo(repo_name)?
            .is_empty()
    );
    harness.shutdown().await;
    Ok(())
}

async fn http_status(harness: &ProxyHarness) -> anyhow::Result<Vec<ChangeEntry>> {
    let response = harness
        .client
        .get(format!("{}/api/sc/status", harness.base_url))
        .query(&[("scope_nonce", "1")])
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    Ok(response.json().await?)
}

async fn post_path(harness: &ProxyHarness, endpoint: &str, path: &str) -> anyhow::Result<()> {
    harness.grant_browser_write(1)?;
    let response = harness
        .client
        .post(format!("{}{}", harness.base_url, endpoint))
        .json(&serde_json::json!({
            "scope_nonce": 1,
            "path": path,
        }))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
    Ok(())
}

async fn post_apply_external_changes(harness: &ProxyHarness) -> anyhow::Result<ExternalApplyReceipt> {
    harness.grant_browser_write(1)?;
    let response = harness
        .client
        .post(format!(
            "{}/api/sc/apply-external-changes",
            harness.base_url
        ))
        .json(&serde_json::json!({
            "scope_nonce": 1,
        }))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "apply external changes failed with body: {body}"
    );
    Ok(serde_json::from_str(&body)?)
}

async fn post_commit(harness: &ProxyHarness, message: &str) -> anyhow::Result<CommitInfo> {
    harness.grant_browser_write(1)?;
    let response = harness
        .client
        .post(format!("{}/api/sc/commit", harness.base_url))
        .json(&serde_json::json!({
            "scope_nonce": 1,
            "message": message,
        }))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "commit failed with body: {body}"
    );
    Ok(serde_json::from_str(&body)?)
}
