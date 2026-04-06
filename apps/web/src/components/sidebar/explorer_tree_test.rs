use super::visible_tree_nodes;
use crate::components::sidebar::tree::build_file_tree;
use deve_core::models::DocId;

#[test]
fn falls_back_to_docs_when_tree_projection_is_empty() {
    let doc_id = DocId::new();
    let nodes = visible_tree_nodes(Vec::new(), vec![(doc_id, "notes/test.md".into())]);
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].path, "notes");
    assert_eq!(nodes[0].children.len(), 1);
    assert_eq!(nodes[0].children[0].doc_id, Some(doc_id));
    assert_eq!(nodes[0].children[0].path, "notes/test.md");
}

#[test]
fn keeps_projected_tree_when_available() {
    let doc_id = DocId::new();
    let projected = visible_tree_nodes(build_file_tree(vec![(doc_id, "a.md".into())]), vec![]);
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].path, "a.md");
}
