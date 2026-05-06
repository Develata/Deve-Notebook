use super::*;

#[test]
fn graph_projection_resolves_wiki_and_markdown_links() {
    let a = DocId::from_u128(1);
    let b = DocId::from_u128(2);
    let docs = vec![
        GraphDocument {
            doc_id: a,
            path: "notes/a.md".to_string(),
            content: "[[b|B Note]] and [B](b.md) and [web](https://example.com)".to_string(),
        },
        GraphDocument {
            doc_id: b,
            path: "notes/b.md".to_string(),
            content: String::new(),
        },
    ];

    let graph = project_documents(&docs);

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 2);
    assert!(graph.edges.iter().all(|edge| edge.to_doc_id == b));
    assert!(
        graph
            .edges
            .iter()
            .all(|edge| edge.target_path == "notes/b.md")
    );
    assert!(graph.unresolved_links.is_empty());
}

#[test]
fn graph_projection_reports_unresolved_without_creating_nodes() {
    let a = DocId::from_u128(1);
    let docs = vec![GraphDocument {
        doc_id: a,
        path: "notes/a.md".to_string(),
        content: "[[missing]]".to_string(),
    }];

    let graph = project_documents(&docs);

    assert_eq!(graph.nodes.len(), 1);
    assert!(graph.edges.is_empty());
    assert_eq!(graph.unresolved_links.len(), 1);
    assert_eq!(graph.unresolved_links[0].target_path, "notes/missing.md");
}

#[test]
fn graph_projection_normalizes_windows_and_parent_paths() {
    let a = DocId::from_u128(1);
    let b = DocId::from_u128(2);
    let docs = vec![
        GraphDocument {
            doc_id: a,
            path: "notes\\daily\\a.md".to_string(),
            content: "[B](../b.md#heading)".to_string(),
        },
        GraphDocument {
            doc_id: b,
            path: "notes/b.md".to_string(),
            content: String::new(),
        },
    ];

    let graph = project_documents(&docs);

    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].target_path, "notes/b.md");
    assert_eq!(graph.edges[0].to_doc_id, b);
}
