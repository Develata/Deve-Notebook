use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
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
    for repo_name in local_repo_names(repo)? {
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

fn local_repo_names(repo: &RepoManager) -> Result<Vec<String>> {
    repo.list_repos(None)
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
        let _ = repo.save_snapshot_in_local_repo(repo_name, doc_id, max_seq, &content);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::local_repo_names;
    use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
    use tempfile::TempDir;

    fn write_info(db: &redb::Database, info: &RepoInfo) {
        let txn = db.begin_write().expect("write txn");
        txn.open_table(REPO_METADATA)
            .expect("repo metadata")
            .insert(&0, bincode::serialize(info).expect("serialize").as_slice())
            .expect("write metadata");
        txn.commit().expect("commit");
    }

    #[test]
    fn local_repo_names_uses_repaired_catalog_names() {
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
        RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki")).expect("wiki");
        let main_info = main.get_repo_info().expect("main info").expect("present");
        let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;
        write_info(
            wiki_db.as_ref(),
            &RepoInfo {
                uuid: main_info.uuid,
                name: "main".into(),
                url: Some(format!("urn:uuid:{}", main_info.uuid)),
            },
        );

        assert_eq!(
            local_repo_names(&main).expect("repo names"),
            vec!["main".to_string(), "wiki".to_string()]
        );
    }
}
