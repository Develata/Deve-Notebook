//! Repo-scoped 目录树注册表。
//!
//! Invariants:
//! - 每个 `(branch, RepoId)` scope 拥有独立的 `TreeManager`。
//! - 任一会话只能读写其当前 repo 对应的树状态。

use deve_core::models::{NodeId, NodeMeta, PeerId, RepoId};
use deve_core::tree::{TreeDelta, TreeManager};
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Clone, Hash, PartialEq, Eq)]
struct TreeScope {
    branch: Option<PeerId>,
    repo_id: RepoId,
}

pub struct RepoTreeRegistry {
    trees: RwLock<HashMap<TreeScope, TreeManager>>,
}

impl RepoTreeRegistry {
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
    ) -> TreeDelta {
        self.with_tree_mut(repo_id, branch, |tree| {
            tree.init_from_nodes(nodes);
            tree.build_init_delta()
        })
    }

    pub fn with_tree_mut<F, R>(&self, repo_id: RepoId, branch: Option<&PeerId>, f: F) -> R
    where
        F: FnOnce(&mut TreeManager) -> R,
    {
        let mut trees = self.trees.write().unwrap_or_else(|e| e.into_inner());
        let tree = trees
            .entry(TreeScope {
                branch: branch.cloned(),
                repo_id,
            })
            .or_default();
        f(tree)
    }
}

impl Default for RepoTreeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::RepoTreeRegistry;
    use deve_core::models::{NodeId, PeerId, RepoId};
    use deve_core::tree::TreeManager;

    #[test]
    fn keeps_local_and_shadow_tree_state_separate() {
        let registry = RepoTreeRegistry::new();
        let repo_id = RepoId::new_v4();
        let node_id = NodeId::new();
        registry.with_tree_mut(repo_id, None, |tree: &mut TreeManager| {
            tree.add_folder(node_id, "notes".into(), None, "notes".into())
        });
        let local_present = registry.with_tree_mut(repo_id, None, |tree| tree.has_node(node_id));
        let remote_present =
            registry.with_tree_mut(repo_id, Some(&PeerId::new("peer-a")), |tree| {
                tree.has_node(node_id)
            });
        assert!(local_present);
        assert!(!remote_present);
    }
}
