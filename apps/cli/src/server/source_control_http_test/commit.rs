//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::{ProxyHarness, path_target, seed_pending, write_workspace_file};
use deve_core::ledger::traits::RepoSelector;
use deve_core::source_control::{ChangeStatus, SourceControlApi};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_commit_queries_roundtrip() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let dir = &harness.dir;
    let repo = harness.repo.clone();
    let proxy = harness.proxy.clone();
    let selector = RepoSelector::default();
    write_workspace_file(dir, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    proxy.stage_pending_in_repo(&selector, &path_target("notes/a.md"))?;
    let c1 = proxy.commit_staged_in_repo(&selector, "c1")?;
    write_workspace_file(dir, "notes/b.md", "world");
    seed_pending(&repo, "notes/b.md", ChangeStatus::Added, "world");
    proxy.stage_pending_in_repo(&selector, &path_target("notes/b.md"))?;
    let c2 = proxy.commit_staged_in_repo(&selector, "c2")?;
    let commits = proxy.list_commits_in_repo(&selector, 10)?;
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].id, c2.id);
    assert_eq!(commits[1].id, c1.id);
    let diffs = proxy.diff_commits_in_repo(&selector, Some(&c1.id), &c2.id)?;
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].doc_id, repo.get_docid("notes/b.md")?,);
    assert_eq!(diffs[0].path, "notes/b.md");
    assert_eq!(diffs[0].status, ChangeStatus::Added);
    harness.shutdown().await;
    Ok(())
}
