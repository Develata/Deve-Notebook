//! plan_ref:
//!   - 10_rendering#large-document-runtime

use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::models::DocId;
use deve_core::state;
use deve_core::sync::snapshot_policy::SnapshotPolicy;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

const PREWARM_LIMIT: usize = 5;

pub fn spawn_prewarm(repo: Arc<RepoManager>) {
    tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        let repo = repo.clone();
        match tokio::task::spawn_blocking(move || prewarm_snapshots(&repo)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("Prewarm snapshots failed: {:?}", e),
            Err(e) => tracing::warn!("Prewarm task panicked: {:?}", e),
        }
    });
}

fn prewarm_snapshots(repo: &RepoManager) -> Result<()> {
    for (repo_name, doc_id) in select_prewarm_docs(repo)? {
        prewarm_doc(repo, &repo_name, doc_id)?;
    }
    Ok(())
}

fn select_prewarm_docs(repo: &RepoManager) -> Result<Vec<(String, DocId)>> {
    let mut per_repo = Vec::new();
    for repo_name in repo.list_local_repo_names_for_execution()? {
        let mut scored = score_repo_docs(repo, &repo_name)?;
        if !scored.is_empty() {
            scored.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            per_repo.push((repo_name, scored));
        }
    }

    let mut selected = Vec::new();
    // Invariant: 选择集合中的 (repo_name, doc_id) 始终属于同一仓库，不跨 repo 混用 doc_id。
    for depth in 0.. {
        let mut progressed = false;
        for (repo_name, docs) in &per_repo {
            if let Some((doc_id, _)) = docs.get(depth) {
                selected.push((repo_name.clone(), *doc_id));
                progressed = true;
                if selected.len() == PREWARM_LIMIT {
                    return Ok(selected);
                }
            }
        }
        if !progressed {
            break;
        }
    }
    Ok(selected)
}

fn score_repo_docs(repo: &RepoManager, repo_name: &str) -> Result<Vec<(DocId, u64)>> {
    let docs = repo.list_local_docs(Some(repo_name))?;
    let mut scored = Vec::new();
    for (doc_id, _) in docs {
        let count = repo.get_local_ops_in_local_repo(repo_name, doc_id)?.len() as u64;
        if count > 0 {
            scored.push((doc_id, count));
        }
    }
    Ok(scored)
}

fn prewarm_doc(repo: &RepoManager, repo_name: &str, doc_id: DocId) -> Result<()> {
    let snapshot = repo.load_latest_snapshot_in_local_repo(repo_name, doc_id)?;
    let base_seq = snapshot.as_ref().map(|(seq, _)| *seq).unwrap_or(0);
    let entries = repo.get_local_ops_in_local_repo(repo_name, doc_id)?;
    let max_seq = entries.last().map(|(seq, _)| *seq).unwrap_or(0);
    let delta = max_seq.saturating_sub(base_seq);
    let doc_len = snapshot
        .as_ref()
        .map(|(_, c)| c.encode_utf16().count())
        .unwrap_or(0);
    let policy = SnapshotPolicy::default();

    if max_seq == 0 {
        return Ok(());
    }

    if snapshot.is_none() || policy.should_snapshot(doc_len, delta, 0) {
        let ops: Vec<_> = entries.iter().map(|(_, e)| e.clone()).collect();
        let content = state::reconstruct_content(&ops);
        repo.save_snapshot_in_local_repo(repo_name, doc_id, max_seq, &content)?;
    }
    Ok(())
}
