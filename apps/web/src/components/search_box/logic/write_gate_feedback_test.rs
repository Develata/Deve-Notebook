use super::write_block_message;

#[test]
fn write_block_message_includes_action_and_reason() {
    assert_eq!(
        write_block_message("move document", "read-only"),
        "Cannot move document: read-only"
    );
}
