//! plan_ref:
//!   - 10_rendering#large-document-runtime

use anyhow::Result;
use deve_core::ledger::{RepoManager, ops};
use deve_core::models::DocId;
use deve_core::state;
use deve_core::sync::snapshot_policy::SnapshotPolicy;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::{Duration, sleep};

const PREWARM_LIMIT: usize = 5;

pub fn spawn_prewarm(
    repo: Arc<RepoManager>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return;
                }
            }
            _ = sleep(Duration::from_secs(2)) => {}
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let blocking_cancelled = cancelled.clone();
        let repo = repo.clone();
        let mut task =
            tokio::task::spawn_blocking(move || prewarm_snapshots(&repo, &blocking_cancelled));
        let result = tokio::select! {
            result = &mut task => result,
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    cancelled.store(true, Ordering::Release);
                }
                task.await
            }
        };
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("Prewarm snapshots failed: {:?}", e),
            Err(e) => tracing::warn!("Prewarm task panicked: {:?}", e),
        }
    })
}

fn prewarm_snapshots(repo: &RepoManager, cancelled: &AtomicBool) -> Result<()> {
    for (repo_name, doc_id) in select_prewarm_docs(repo)? {
        if cancelled.load(Ordering::Acquire) {
            return Ok(());
        }
        prewarm_doc(repo, &repo_name, doc_id, cancelled)?;
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
        let count = repo.run_on_local_repo(repo_name, |db| ops::count_ops_from_db(db, doc_id))?;
        if count > 0 {
            scored.push((doc_id, count));
        }
    }
    Ok(scored)
}

fn prewarm_doc(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
    cancelled: &AtomicBool,
) -> Result<()> {
    let snapshot = repo.load_latest_snapshot_in_local_repo(repo_name, doc_id)?;
    let base_seq = snapshot.as_ref().map(|(seq, _)| *seq).unwrap_or(0);
    let max_seq = repo.run_on_local_repo(repo_name, |db| ops::max_seq_from_db(db, doc_id))?;
    let delta = max_seq.saturating_sub(base_seq);
    let doc_len = snapshot
        .as_ref()
        .map(|(_, c)| c.encode_utf16().count())
        .unwrap_or(0);
    let policy = SnapshotPolicy::default();

    if max_seq == 0 {
        return Ok(());
    }

    if snapshot_rebuild_required(snapshot.is_some(), doc_len, delta, max_seq, policy) {
        let entries = repo.get_local_ops_in_local_repo(repo_name, doc_id)?;
        let ops: Vec<_> = entries.into_iter().map(|(_, entry)| entry).collect();
        let Some(content) =
            state::reconstruct_content_until(&ops, || cancelled.load(Ordering::Acquire))
        else {
            return Ok(());
        };
        if cancelled.load(Ordering::Acquire) {
            return Ok(());
        }
        repo.save_snapshot_in_local_repo(repo_name, doc_id, max_seq, &content)?;
    }
    Ok(())
}

fn snapshot_rebuild_required(
    has_snapshot: bool,
    doc_len: usize,
    delta: u64,
    max_seq: u64,
    policy: SnapshotPolicy,
) -> bool {
    max_seq > 0 && (!has_snapshot || policy.should_snapshot(doc_len, delta, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prewarm_up_to_date_snapshot_skips_history_rebuild() {
        assert!(!snapshot_rebuild_required(
            true,
            128,
            0,
            42,
            SnapshotPolicy::default(),
        ));
    }

    #[test]
    fn prewarm_missing_snapshot_requires_history_rebuild() {
        assert!(snapshot_rebuild_required(
            false,
            0,
            0,
            42,
            SnapshotPolicy::default(),
        ));
        assert!(!snapshot_rebuild_required(
            false,
            0,
            0,
            0,
            SnapshotPolicy::default(),
        ));
    }
}
