use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::sync::snapshot_policy::SnapshotPolicy;
use deve_core::state;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

const PREWARM_LIMIT: usize = 5;

pub fn spawn_prewarm(repo: Arc<RepoManager>) {
    tokio::spawn(async move {
        sleep(Duration::from_secs(2)).await;
        let repo = repo.clone();
        let _ = tokio::task::spawn_blocking(move || prewarm_snapshots(&repo)).await;
    });
}

fn prewarm_snapshots(repo: &RepoManager) -> Result<()> {
    let docs = repo.list_local_docs(None)?;
    let mut scored = Vec::new();

    for (doc_id, _path) in docs {
        let entries = repo.get_local_ops(doc_id)?;
        let count = entries.len() as u64;
        if count > 0 {
            scored.push((doc_id, count));
        }
    }

    scored.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    for (doc_id, _count) in scored.into_iter().take(PREWARM_LIMIT) {
        let snapshot = repo.load_latest_snapshot(doc_id)?;
        let base_seq = snapshot.as_ref().map(|(seq, _)| *seq).unwrap_or(0);
        let entries = repo.get_local_ops(doc_id)?;
        let max_seq = entries.last().map(|(seq, _)| *seq).unwrap_or(0);
        let delta = max_seq.saturating_sub(base_seq);
        let doc_len = snapshot
            .as_ref()
            .map(|(_, c)| c.encode_utf16().count())
            .unwrap_or(0);
        let policy = SnapshotPolicy::default();

        if max_seq == 0 {
            continue;
        }

        if snapshot.is_none() || policy.should_snapshot(doc_len, delta, 0) {
            let ops: Vec<_> = entries.iter().map(|(_, e)| e.clone()).collect();
            let content = state::reconstruct_content(&ops);
            let _ = repo.save_snapshot(doc_id, max_seq, &content);
        }
    }

    Ok(())
}
