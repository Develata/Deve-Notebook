use deve_core::models::NodeId;
use deve_core::tree::FileNode;

pub(super) fn sort_nodes(nodes: &mut [FileNode]) {
    nodes.sort_by(|a, b| match (a.doc_id.is_none(), b.doc_id.is_none()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
}

pub(super) fn find_node_mut(roots: &mut [FileNode], node_id: NodeId) -> Option<&mut FileNode> {
    for node in roots.iter_mut() {
        if node.node_id == node_id {
            return Some(node);
        }
        if let Some(found) = find_node_mut(&mut node.children, node_id) {
            return Some(found);
        }
    }
    None
}

pub(super) fn find_node_mut_by_path<'a>(
    roots: &'a mut [FileNode],
    path: &str,
) -> Option<&'a mut FileNode> {
    for node in roots.iter_mut() {
        if normalize_path(&node.path) == normalize_path(path) {
            return Some(node);
        }
        if let Some(found) = find_node_mut_by_path(&mut node.children, path) {
            return Some(found);
        }
    }
    None
}

pub(super) fn parent_path_of(path: &str) -> Option<&str> {
    path.rsplit_once('/').map(|(parent, _)| parent)
}

pub(super) fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}
