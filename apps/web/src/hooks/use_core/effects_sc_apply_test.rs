use super::file_op_feedback_message;

#[test]
fn file_op_feedback_messages_cover_doc_mutations() {
    assert_eq!(
        file_op_feedback_message("notes/a.md", "added").as_deref(),
        Some("Created: notes/a.md")
    );
    assert_eq!(
        file_op_feedback_message("notes/a.md", "dir-added").as_deref(),
        Some("Created folder: notes/a.md")
    );
    assert_eq!(
        file_op_feedback_message("notes/a.md", "renamed").as_deref(),
        Some("Renamed: notes/a.md")
    );
    assert_eq!(
        file_op_feedback_message("notes/a.md", "deleted").as_deref(),
        Some("Deleted: notes/a.md")
    );
    assert_eq!(
        file_op_feedback_message("notes/a.md", "copied").as_deref(),
        Some("Copied: notes/a.md")
    );
}

#[test]
fn file_op_feedback_ignores_background_refreshes() {
    assert_eq!(file_op_feedback_message("notes/a.md", "dir_changed"), None);
}
