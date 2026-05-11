use deve_core::models::{DocId, NodeId};
use deve_core::tree::{FileNode, TreeDelta, TreeManager};

fn child<'a>(nodes: &'a [FileNode], name: &str) -> &'a FileNode {
    nodes
        .iter()
        .find(|node| node.name == name)
        .expect("child node exists")
}

#[test]
fn rename_updates_descendant_paths_by_subtree() {
    let mut tree = TreeManager::new();
    let root = NodeId::new();
    let dir = NodeId::new();
    let doc_id = DocId::new();
    let file_node = NodeId::from_doc_id(doc_id);

    tree.add_folder(root, "old".into(), None, "old".into());
    tree.add_folder(dir, "old/inner".into(), Some(root), "inner".into());
    tree.add_file(
        file_node,
        "old/inner/a.md".into(),
        Some(dir),
        "a.md".into(),
        doc_id,
    );

    tree.rename_node(root, "new".into());
    let TreeDelta::Init { roots } = tree.build_init_delta() else {
        panic!("expected init delta");
    };
    let root = child(&roots, "new");
    let dir = child(&root.children, "inner");
    let file = child(&dir.children, "a.md");

    assert_eq!(root.path, "new");
    assert_eq!(dir.path, "new/inner");
    assert_eq!(file.path, "new/inner/a.md");
}

#[test]
fn move_updates_descendant_paths_by_subtree() {
    let mut tree = TreeManager::new();
    let src = NodeId::new();
    let dst = NodeId::new();
    let dir = NodeId::new();
    let doc_id = DocId::new();
    let file_node = NodeId::from_doc_id(doc_id);

    tree.add_folder(src, "src".into(), None, "src".into());
    tree.add_folder(dst, "dst".into(), None, "dst".into());
    tree.add_folder(dir, "src/inner".into(), Some(src), "inner".into());
    tree.add_file(
        file_node,
        "src/inner/a.md".into(),
        Some(dir),
        "a.md".into(),
        doc_id,
    );

    tree.move_node(dir, Some(dst));
    let TreeDelta::Init { roots } = tree.build_init_delta() else {
        panic!("expected init delta");
    };
    let dst = child(&roots, "dst");
    let dir = child(&dst.children, "inner");
    let file = child(&dir.children, "a.md");

    assert_eq!(dir.path, "dst/inner");
    assert_eq!(file.path, "dst/inner/a.md");
    assert!(child(&roots, "src").children.is_empty());
}
