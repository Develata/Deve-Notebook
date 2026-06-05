use super::*;

#[test]
fn file_tree_action_target_kind_derives_from_file_tree_node() {
    assert_eq!(
        ContextActionTarget::from_file_tree_node(true, "notes/readme.md"),
        ContextActionTarget::new(ContextActionTargetKind::Folder, "notes/readme.md")
    );
    assert_eq!(
        ContextActionTarget::from_file_tree_node(false, "notes/readme.md"),
        ContextActionTarget::new(ContextActionTargetKind::MarkdownFile, "notes/readme.md")
    );
    assert_eq!(
        ContextActionTarget::from_file_tree_node(false, "notes\\README.MARKDOWN"),
        ContextActionTarget::new(
            ContextActionTargetKind::MarkdownFile,
            "notes/README.MARKDOWN"
        )
    );
    assert_eq!(
        ContextActionTarget::from_file_tree_node(false, "assets/image.png").kind,
        ContextActionTargetKind::File
    );
}

#[test]
fn file_tree_action_target_kind_matching_keeps_markdown_as_file_but_not_folder() {
    assert!(ContextActionTargetKind::File.accepts(ContextActionTargetKind::MarkdownFile));
    assert!(ContextActionTargetKind::AnyNode.accepts(ContextActionTargetKind::Folder));
    assert!(!ContextActionTargetKind::Folder.accepts(ContextActionTargetKind::File));
}

#[test]
fn file_tree_action_export_pdf_matches_markdown_target_only() {
    let export = context_action_by_id(ContextActionId::ExportPdf).expect("export pdf action");

    assert!(
        export
            .target_kind
            .accepts(ContextActionTargetKind::MarkdownFile)
    );
    assert!(!export.target_kind.accepts(ContextActionTargetKind::File));
    assert!(!export.target_kind.accepts(ContextActionTargetKind::Folder));
}
