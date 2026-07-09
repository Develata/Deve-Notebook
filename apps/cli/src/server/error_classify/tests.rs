//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Shared server error classification tests.

use super::is_storage_corruption;

#[test]
fn classifies_remote_peer_directory_stat_errors_as_storage_corruption() {
    assert!(is_storage_corruption(
        "failed to stat remote peer directory \"/tmp/remotes/peer-a\" while validating branch availability: permission denied",
    ));
}

#[test]
fn classifies_remote_peer_directory_metadata_errors_as_storage_corruption() {
    assert!(is_storage_corruption(
        "failed to read remote peer directory metadata \"/tmp/remotes/peer-a\" while validating branch availability: input/output error",
    ));
}

#[test]
fn classifies_tree_registry_poison_as_storage_corruption() {
    assert!(is_storage_corruption(
        "repotreeregistry write lock poisoned while rebuilding repo view",
    ));
}

#[test]
fn classifies_registry_poison_as_storage_corruption() {
    assert!(is_storage_corruption("shadow db registry lock poisoned"));
    assert!(is_storage_corruption(
        "database cache lock poisoned while storing /tmp/repo.redb",
    ));
    assert!(is_storage_corruption(
        "reposcopedsyncengine write lock poisoned",
    ));
}

#[test]
fn classifies_local_repo_catalog_metadata_corruption() {
    assert!(is_storage_corruption(
        "failed to list local repos: local repo notes metadata name drifted to peer-remote",
    ));
    assert!(is_storage_corruption(
        "failed to list local repos: postcard deserialization failed: serde deserialization error",
    ));
}
