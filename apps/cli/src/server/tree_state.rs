//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!   - 06_repository#tree-projection-contract
//!
//! Repo-scoped 目录树注册表。
//!
//! Invariants:
//! - 每个 `(branch, RepoId)` scope 拥有独立的 `TreeManager`。
//! - 任一会话只能读写其当前 repo 对应的树状态。

use anyhow::{Result, anyhow};
use deve_core::models::{NodeId, NodeMeta, PeerId, RepoId, StructureOp};
use deve_core::tree::{TreeDelta, TreeManager, tree_delta_bridge};
use std::collections::HashMap;
use std::sync::{RwLock, RwLockWriteGuard};

#[derive(Clone, Hash, PartialEq, Eq)]
struct TreeScope {
    branch: Option<PeerId>,
    repo_id: RepoId,
}

pub struct RepoTreeRegistry {
    trees: RwLock<HashMap<TreeScope, TreeManager>>,
}

impl RepoTreeRegistry {
    fn write_trees(&self) -> Result<RwLockWriteGuard<'_, HashMap<TreeScope, TreeManager>>> {
        self.trees
            .write()
            .map_err(|_| anyhow!("RepoTreeRegistry write lock poisoned"))
    }

    pub fn new() -> Self {
        Self {
            trees: RwLock::new(HashMap::new()),
        }
    }

    pub fn reset_from_nodes(
        &self,
        repo_id: RepoId,
        branch: Option<&PeerId>,
        nodes: Vec<(NodeId, NodeMeta)>,
    ) -> Result<TreeDelta> {
        self.with_tree_mut(repo_id, branch, |tree| {
            tree.init_from_nodes(nodes);
            tree.build_init_delta()
        })
    }

    pub fn apply_structure_ops(
        &self,
        repo_id: RepoId,
        branch: Option<&PeerId>,
        ops: &[StructureOp],
    ) -> Result<Vec<TreeDelta>> {
        self.with_tree_mut(repo_id, branch, |tree| {
            tree_delta_bridge::apply_structure_ops(tree, ops)
        })
    }

    pub fn with_tree_mut<F, R>(&self, repo_id: RepoId, branch: Option<&PeerId>, f: F) -> Result<R>
    where
        F: FnOnce(&mut TreeManager) -> R,
    {
        let mut trees = self.write_trees()?;
        let tree = trees
            .entry(TreeScope {
                branch: branch.cloned(),
                repo_id,
            })
            .or_default();
        Ok(f(tree))
    }
}

impl Default for RepoTreeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
