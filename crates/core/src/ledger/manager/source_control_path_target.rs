use crate::ledger::RepoManager;
use crate::protocol::ScPathTarget;
use crate::utils::path::to_forward_slash;
use anyhow::Result;

impl RepoManager {
    /// 将旧的 path-only Source Control 入口提升为 tracked target。
    ///
    /// Invariants:
    /// - 若当前 path 已被 node projection 跟踪，则必须补上 `doc_id`。
    /// - 若 path 不是当前 tracked projection，只保留规范化 path，不猜测旧 mapping。
    pub(super) fn tracked_target_for_path_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<ScPathTarget> {
        let path = to_forward_slash(path);
        Ok(ScPathTarget {
            doc_id: self.get_tracked_docid_in_local_repo(repo_name, &path)?,
            path,
        })
    }
}
