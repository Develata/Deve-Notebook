//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::super::support::{
    default_workspace_root, path_target, seed_pending, seed_tracked_rename, write_workspace_file,
    ProxyHarness,
};
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeStatus, SourceControlApi};

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
    proxy.apply_external_changes_in_repo(&selector)?;
    proxy.commit_source_control_changes_in_repo(&selector, "initial")?;
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
            domain: None,
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
    repo.apply_external_changes_in_repo(&selector)?;
    repo.commit_source_control_changes_in_repo(&selector, "initial")?;
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
