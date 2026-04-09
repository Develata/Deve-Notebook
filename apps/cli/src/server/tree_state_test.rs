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
