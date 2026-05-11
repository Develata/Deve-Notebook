//! plan_ref:
//!   - 04_storage#watcher-contract
#![allow(dead_code)]
use anyhow::{Result, anyhow};
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, PeerId, RepoId};
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use deve_core::sync::{SyncManager, watcher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

use crate::common;

pub struct Harness {
    pub dir: TempDir,
    pub repo: Arc<RepoManager>,
    pub sync: Arc<SyncManager>,
    watcher_ids: Vec<RepoId>,
}

impl Harness {
    pub fn new(extra_repo: Option<(&str, &str)>) -> Result<Self> {
        let dir = TempDir::new()?;
        let ledger = dir.path().join("ledger");
        std::fs::create_dir_all(dir.path().join("vault"))?;
        let mut repo = RepoManager::init(&ledger, 10, Some("main"), Some("urn:main"))?;
        if let Some((name, url)) = extra_repo {
            let _ = common::try_create_initialized_local_repo_with_depth(&ledger, 10, name, url)?;
        }
        repo.set_vault_root_checked(dir.path().join("vault"))?;
        let repo = Arc::new(repo);
        let sync = Arc::new(SyncManager::new_checked(
            repo.clone(),
            dir.path().join("vault"),
        )?);
        Ok(Self {
            dir,
            repo,
            sync,
            watcher_ids: Vec::new(),
        })
    }

    pub fn start_watchers(&mut self) -> Result<()> {
        for repo_name in self.repo.list_local_repo_names_for_execution()? {
            self.watcher_ids.push(watcher::start_repo_watcher(
                self.sync.clone(),
                &repo_name,
                None,
                None,
            )?);
        }
        std::thread::sleep(Duration::from_millis(200));
        Ok(())
    }

    pub fn commit_doc(&self, repo_name: &str, path: &str, content: &str) -> Result<DocId> {
        let (doc_id, _) = self
            .repo
            .apply_file_structure_in_local_repo(repo_name, path, None, "test")?;
        self.repo.append_generated_op_in_local_repo(
            repo_name,
            doc_id,
            PeerId::new("local"),
            |seq| {
                LedgerEntry::new_content(
                    doc_id,
                    deve_core::models::Op::Insert {
                        pos: 0,
                        content: content.into(),
                    },
                    1,
                    PeerId::new("local"),
                    seq,
                    None,
                    None,
                )
            },
        )?;
        self.sync.persist_doc_in_local_repo(repo_name, doc_id)?;
        Ok(doc_id)
    }

    pub fn wait_pending(
        &self,
        repo_name: &str,
        path: &str,
        status: ChangeStatus,
    ) -> Result<ChangeEntry> {
        self.wait_until(Duration::from_secs(5), || {
            self.repo
                .list_pending_fs_in_local_repo(repo_name)
                .ok()?
                .into_iter()
                .find(|entry| entry.path == path && entry.status == status)
        })
        .map_err(|err| anyhow!("pending {repo_name}/{path} {status:?} not observed: {err}"))
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
        for repo_id in self.watcher_ids.drain(..) {
            let _ = watcher::stop_repo_watcher(repo_id);
        }
    }
}
