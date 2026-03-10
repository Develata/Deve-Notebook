//! Repo-scoped 目录树注册表。
//!
//! Invariants:
//! - 每个 `RepoId` 拥有独立的 `TreeManager`。
//! - 任一会话只能读写其当前 repo 对应的树状态。

use deve_core::models::{NodeId, NodeMeta, RepoId};
use deve_core::tree::{TreeDelta, TreeManager};
use std::collections::HashMap;
use std::sync::RwLock;

pub struct RepoTreeRegistry {
    trees: RwLock<HashMap<RepoId, TreeManager>>,
}

impl RepoTreeRegistry {
    pub fn new() -> Self {
        Self {
            trees: RwLock::new(HashMap::new()),
        }
    }

    pub fn reset_from_nodes(&self, repo_id: RepoId, nodes: Vec<(NodeId, NodeMeta)>) -> TreeDelta {
        self.with_tree_mut(repo_id, |tree| {
            tree.init_from_nodes(nodes);
            tree.build_init_delta()
        })
    }

    pub fn with_tree_mut<F, R>(&self, repo_id: RepoId, f: F) -> R
    where
        F: FnOnce(&mut TreeManager) -> R,
    {
        let mut trees = self.trees.write().unwrap_or_else(|e| e.into_inner());
        let tree = trees.entry(repo_id).or_default();
        f(tree)
    }
}

impl Default for RepoTreeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
