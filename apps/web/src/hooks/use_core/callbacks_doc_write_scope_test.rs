use super::write_block_banner;

#[test]
fn write_block_banner_includes_action_and_reason() {
    assert_eq!(
        write_block_banner("DeleteDoc", "offline"),
        "Cannot send DeleteDoc: offline"
    );
}
