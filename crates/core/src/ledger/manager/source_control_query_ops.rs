// crates/core/src/ledger/manager/source_control_query_ops.rs
//! # 版本控制查询
//!
//! 提供未提交变更与 Diff 相关查询。

use crate::ledger::RepoManager;
use crate::ledger::metadata;
use crate::source_control::ChangeEntry;
use crate::source_control::diff;
use crate::source_control::snapshot_paths;
use crate::state::reconstruct_content;
use crate::utils::path::to_forward_slash;
use anyhow::Result;

impl RepoManager {
    /// 获取未提交的文件变更列表 (基于快照对比)
    pub fn list_changes(&self) -> Result<Vec<ChangeEntry>> {
        let docs = metadata::list_docs(&self.local_db)?;
        let snapshot_paths = snapshot_paths::list_snapshot_paths(&self.local_db)?;
        let mut current_map = std::collections::HashMap::new();
        for (doc_id, path) in &docs {
            current_map.insert(*doc_id, path.clone());
        }
        let mut changes = Vec::new();

        for (doc_id, path) in docs {
            let committed = self.get_committed_content(doc_id)?;
            let ops = self.get_local_ops(doc_id)?;
            let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
            let current = reconstruct_content(&entries);

            if let Some(status) = self.detect_change(committed.as_deref(), Some(&current)) {
                changes.push(ChangeEntry { path, status });
            }
        }

        for (doc_id, path) in snapshot_paths {
            if current_map.contains_key(&doc_id) {
                continue;
            }
            let committed = self.get_committed_content(doc_id)?;
            if let Some(status) = self.detect_change(committed.as_deref(), None) {
                changes.push(ChangeEntry { path, status });
            }
        }

        Ok(changes)
    }

    /// 生成指定路径的统一 Diff (基于快照对比)
    pub fn diff_doc_path(&self, path: &str) -> Result<String> {
        let normalized = to_forward_slash(path);
        let doc_id = metadata::get_docid(&self.local_db, &normalized)?
            .or_else(|| {
                snapshot_paths::find_snapshot_doc_id(&self.local_db, &normalized)
                    .ok()
                    .flatten()
            })
            .ok_or_else(|| anyhow::anyhow!("Doc not found: {}", normalized))?;

        let committed = self.get_committed_content(doc_id)?;
        let current = if metadata::get_docid(&self.local_db, &normalized)?.is_some() {
            let ops = self.get_local_ops(doc_id)?;
            let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
            reconstruct_content(&entries)
        } else {
            String::new()
        };

        let old = committed.as_deref().unwrap_or("");
        Ok(diff::unified_diff(old, &current, &normalized))
    }
}
