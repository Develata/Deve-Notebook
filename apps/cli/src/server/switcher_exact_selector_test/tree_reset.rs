//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_repo;
use crate::server::switcher_test_support::{app_state_with_tree, browser_session, unicast_channel};
use crate::server::tree_state::RepoTreeRegistry;
use deve_core::ledger::RepoManager;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_repo_does_not_emit_partial_repo_view_when_tree_reset_fails() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let tree_manager = Arc::new(RepoTreeRegistry::new());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = tree_manager.with_tree_mut(uuid::Uuid::new_v4(), None, |_| {
            panic!("poison tree registry")
        });
    }));
    let state = app_state_with_tree(repo, vault, dir.path().join("host"), tree_manager)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(10);

    handle_switch_repo(&state, &ch, &mut session, "default".into(), None, Some(11)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(switch_nonce, Some(11));
        }
        other => panic!("expected tree rebuild ProtocolError, got {:?}", other),
    }
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(
        uni_rx.try_recv().is_err(),
        "must not emit partial repo view"
    );
    Ok(())
}
