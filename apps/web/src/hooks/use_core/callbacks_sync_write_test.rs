use super::sync_write_block_banner;

#[test]
fn sync_write_block_banner_includes_action_and_reason() {
    assert_eq!(
        sync_write_block_banner("ConfirmMerge", "repo handshaking"),
        "Cannot send ConfirmMerge: repo handshaking"
    );
}
