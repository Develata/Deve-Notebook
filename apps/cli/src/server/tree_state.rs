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
mod tests {
    use super::RepoTreeRegistry;
    use deve_core::models::{NodeId, PeerId, RepoId};
    use deve_core::tree::TreeManager;

    #[test]
    fn keeps_local_and_shadow_tree_state_separate() {
        let registry = RepoTreeRegistry::new();
        let repo_id = RepoId::new_v4();
        let node_id = NodeId::new();
        registry
            .with_tree_mut(repo_id, None, |tree: &mut TreeManager| {
                tree.add_folder(node_id, "notes".into(), None, "notes".into())
            })
            .expect("local tree update");
        let local_present = registry
            .with_tree_mut(repo_id, None, |tree| tree.has_node(node_id))
            .expect("local tree read");
        let remote_present = registry
            .with_tree_mut(repo_id, Some(&PeerId::new("peer-a")), |tree| {
                tree.has_node(node_id)
            })
            .expect("remote tree read");
        assert!(local_present);
        assert!(!remote_present);
    }

    #[test]
    fn fails_closed_when_tree_registry_lock_is_poisoned() {
        let registry = RepoTreeRegistry::new();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = registry.trees.write().expect("write lock");
            panic!("poison tree registry");
        }));

        let err = registry
            .reset_from_nodes(RepoId::new_v4(), None, vec![])
            .expect_err("poisoned registry must fail closed");
        assert!(err.to_string().contains("lock poisoned"));
    }
}
