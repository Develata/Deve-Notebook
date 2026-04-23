use super::create_block_banner;

#[test]
fn create_block_banner_includes_reason() {
    assert_eq!(
        create_block_banner("snapshot loading"),
        "Cannot create document: snapshot loading"
    );
}
