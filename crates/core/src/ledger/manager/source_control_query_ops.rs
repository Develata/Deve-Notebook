// crates/core/src/ledger/manager/source_control_query_ops.rs
//! # 版本控制查询
//!
//! 提供未提交变更与 Diff 相关查询。

use crate::ledger::RepoManager;
use crate::ledger::metadata;
use crate::source_control::diff;
use crate::source_control::snapshot_paths;
use crate::source_control::{ChangeEntry, CommitFileDiff};
use crate::state::reconstruct_content;
use crate::utils::path::to_forward_slash;
use anyhow::Result;

impl RepoManager {
    /// 获取未提交的文件变更列表 (基于快照对比)
    pub fn list_changes(&self) -> Result<Vec<ChangeEntry>> {
        self.list_changes_in_local_repo(self.local_repo_name())
    }

    pub fn list_changes_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.run_on_local_repo(repo_name, |db| {
            let docs = metadata::list_docs(db)?;
            let snapshot_paths = snapshot_paths::list_snapshot_paths(db)?;
            let mut current_map = std::collections::HashMap::new();
            for (doc_id, path) in &docs {
                current_map.insert(*doc_id, path.clone());
            }
            let mut changes = Vec::new();

            for (doc_id, path) in docs {
                let committed = crate::ledger::source_control::get_committed_content(db, doc_id)?;
                let ops = crate::ledger::ops::get_ops_from_db(db, doc_id)?;
                let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
                let current = reconstruct_content(&entries);

                if let Some(status) = self.detect_change(committed.as_deref(), Some(&current)) {
                    changes.push(ChangeEntry {
                        path,
                        status,
                        has_conflict: false,
                    });
                }
            }

            for (doc_id, path) in snapshot_paths {
                if current_map.contains_key(&doc_id) {
                    continue;
                }
                let committed = crate::ledger::source_control::get_committed_content(db, doc_id)?;
                if let Some(status) = self.detect_change(committed.as_deref(), None) {
                    changes.push(ChangeEntry {
                        path,
                        status,
                        has_conflict: false,
                    });
                }
            }

            Ok(changes)
        })
    }

    /// 生成指定路径的统一 Diff (基于快照对比)
    pub fn diff_doc_path(&self, path: &str) -> Result<String> {
        self.diff_doc_path_in_local_repo(self.local_repo_name(), path)
    }

    pub fn diff_doc_path_in_local_repo(&self, repo_name: &str, path: &str) -> Result<String> {
        self.run_on_local_repo(repo_name, |db| {
            let normalized = to_forward_slash(path);
            let doc_id = metadata::get_docid(db, &normalized)?
                .or_else(|| {
                    snapshot_paths::find_snapshot_doc_id(db, &normalized)
                        .ok()
                        .flatten()
                })
                .ok_or_else(|| anyhow::anyhow!("Doc not found: {}", normalized))?;

            let committed = crate::ledger::source_control::get_committed_content(db, doc_id)?;
            let current = if metadata::get_docid(db, &normalized)?.is_some() {
                let ops = crate::ledger::ops::get_ops_from_db(db, doc_id)?;
                let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
                reconstruct_content(&entries)
            } else {
                String::new()
            };

            let old = committed.as_deref().unwrap_or("");
            Ok(diff::unified_diff(old, &current, &normalized))
        })
    }

    /// 对比两个提交之间的文件差异 (SC-003)
    ///
    /// **参数**:
    /// - `commit_a_id`: 较早提交 ID (None = 空状态，即查看首次提交的全量变更)
    /// - `commit_b_id`: 较新提交 ID
    pub fn diff_commits(
        &self,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        self.diff_commits_in_local_repo(self.local_repo_name(), commit_a_id, commit_b_id)
    }

    pub fn diff_commits_in_local_repo(
        &self,
        repo_name: &str,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        let commit_a = commit_a_id.map(str::to_owned);
        let commit_b = commit_b_id.to_owned();
        self.run_on_local_repo(repo_name, |db| {
            crate::source_control::commit_diff::compare_commits(db, commit_a.as_deref(), &commit_b)
        })
    }
}
