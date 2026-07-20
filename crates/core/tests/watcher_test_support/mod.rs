//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
#![allow(dead_code)]
use anyhow::{Result, anyhow};
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry};
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use deve_core::sync::{SyncManager, watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

use crate::common;

pub struct Harness {
    pub dir: TempDir,
    pub repo: Arc<RepoManager>,
    pub sync: Arc<SyncManager>,
    /// Friendly test label (e.g. "main"/"wiki") -> canonical execution name
    /// (the repo's RepoId string). Machine names are UUID-canonical after the
    /// catalog cutover; the labels only exist for test readability.
    labels: HashMap<String, String>,
    /// Extra cataloged repos are created through their own managers on the same
    /// ledger; keep them alive so their process-cached databases stay open.
    _extra_repos: Vec<RepoManager>,
    watcher_handles: Vec<watcher::RepoWatcherHandle>,
}

impl Harness {
    pub fn new(extra_repo: Option<(&str, &str)>) -> Result<Self> {
        let dir = TempDir::new()?;
        let ledger = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        std::fs::create_dir_all(&projection_base)?;
        let (main_repo, main_id) = common::init_cataloged_repo(&ledger, &projection_base)?;
        let mut labels = HashMap::new();
        labels.insert("main".to_string(), main_id.to_string());
        let mut extra_repos = Vec::new();
        if let Some((label, url)) = extra_repo {
            let (extra, extra_id) =
                common::init_cataloged_repo_with_url(&ledger, &projection_base, url)?;
            labels.insert(label.to_string(), extra_id.to_string());
            extra_repos.push(extra);
        }
        let repo = Arc::new(main_repo);
        let sync = Arc::new(SyncManager::new_checked(repo.clone())?);
        Ok(Self {
            dir,
            repo,
            sync,
            labels,
            _extra_repos: extra_repos,
            watcher_handles: Vec::new(),
        })
    }

    /// Translate a friendly test label into the repo's canonical execution name
    /// (its RepoId string). Inputs that are not registered labels — such as an
    /// execution name a test already resolved via `repo.local_repo_name()` —
    /// pass through unchanged.
    pub fn repo_name(&self, label: &str) -> String {
        self.labels
            .get(label)
            .cloned()
            .unwrap_or_else(|| label.to_string())
    }

    pub fn start_watchers(&mut self) -> Result<()> {
        for repo_name in self.repo.list_local_repo_names_for_execution()? {
            self.watcher_handles.push(watcher::RepoWatcherHandle::start(
                watcher::RepoWatcherStart::resolve(self.sync.clone(), repo_name, 1)?,
            )?);
        }
        std::thread::sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn commit_doc(&self, repo_name: &str, path: &str, content: &str) -> Result<DocId> {
        let repo_name = self.repo_name(repo_name);
        let (doc_id, _) = self
            .repo
            .apply_file_structure_in_local_repo(&repo_name, path, None, "test")?;
        self.repo.append_generated_op_in_local_repo(
            &repo_name,
            doc_id,
            self.repo.local_peer_id().clone(),
            |seq| {
                LedgerEntry::new_content(
                    doc_id,
                    deve_core::models::Op::Insert {
                        pos: 0,
                        content: content.into(),
                    },
                    1,
                    self.repo.local_peer_id().clone(),
                    seq,
                    None,
                    None,
                )
            },
        )?;
        self.sync.persist_doc_in_local_repo(&repo_name, doc_id)?;
        Ok(doc_id)
    }

    pub fn wait_pending(
        &self,
        repo_name: &str,
        path: &str,
        status: ChangeStatus,
    ) -> Result<ChangeEntry> {
        let repo_name = self.repo_name(repo_name);
        let result = self.wait_until(Duration::from_secs(5), || {
            self.repo
                .list_pending_fs_in_local_repo(&repo_name)
                .ok()?
                .into_iter()
                .find(|entry| entry.path == path && entry.status == status)
        });
        result.map_err(|err| {
            let pending = self
                .repo
                .list_pending_fs_in_local_repo(&repo_name)
                .unwrap_or_default();
            anyhow!(
                "pending {repo_name}/{path} {status:?} not observed: {err}; pending={pending:?}"
            )
        })
    }

    pub fn workspace_root(&self, repo_name: &str) -> Result<PathBuf> {
        self.repo
            .local_repo_workspace_root(&self.repo_name(repo_name))
    }

    pub fn workspace_path(&self, repo_name: &str, repo_path: &str) -> Result<PathBuf> {
        self.repo
            .local_repo_workspace_path(&self.repo_name(repo_name), repo_path)
    }

    fn wait_until<T>(&self, timeout: Duration, mut f: impl FnMut() -> Option<T>) -> Result<T> {
        let start = Instant::now();
        loop {
            if let Some(value) = f() {
                return Ok(value);
            }
            if start.elapsed() >= timeout {
                return Err(anyhow!("timeout after {:?}", timeout));
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        while let Some(handle) = self.watcher_handles.pop() {
            let _ = handle.shutdown();
        }
    }
}
