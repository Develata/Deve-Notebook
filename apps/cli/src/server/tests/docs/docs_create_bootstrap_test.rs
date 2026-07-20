//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 04_repository#repo-scope-runtime

use super::docs_test_support::{browser_session, channel, docs_harness, stale_db_handle};
use super::{handlers::docs::handle_create_doc, session::WsSession};
use deve_core::models::PeerId;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_ignores_stale_remote_readonly_binding_after_scope_recovery(
) -> anyhow::Result<()> {
    let h = docs_harness()?;
    let (ch, _rx) = channel(&h.state);
    let mut session = browser_session(&h.state, h.repo_id, 31);
    session.set_active_db(stale_db_handle(
        h.dir.path().join("stale-remote.redb"),
        true,
        Some(PeerId::new("remote")),
        "shadow",
    )?);

    handle_create_doc(&h.state, &ch, &mut session, "notes/local.md".into()).await;

    assert!(session.get_active_db().is_none());
    assert!(h.state.repo.get_docid("notes/local.md")?.is_some());
    assert!(h.workspace_path("notes/local.md").exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_without_repo_selection_bootstraps_single_repo() -> anyhow::Result<()> {
    let h = docs_harness()?;
    let (ch, _rx) = channel(&h.state);
    let mut session = WsSession::new();

    handle_create_doc(&h.state, &ch, &mut session, "notes/bootstrapped.md".into()).await;

    assert_eq!(
        session.active_repo.as_deref(),
        Some(h.state.repo.local_repo_name())
    );
    assert_eq!(session.active_repo_id, Some(h.repo_id));
    assert!(h.state.repo.get_docid("notes/bootstrapped.md")?.is_some());
    assert!(h.workspace_path("notes/bootstrapped.md").exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_with_stale_local_binding_bootstraps_single_repo() -> anyhow::Result<()> {
    let h = docs_harness()?;
    let (ch, _rx) = channel(&h.state);
    let mut session = WsSession::new();
    session.set_active_db(stale_db_handle(
        h.dir.path().join("stale-local.redb"),
        false,
        None,
        "ghost",
    )?);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(uuid::Uuid::new_v4());
    session.set_sync_scope_nonce(33);

    handle_create_doc(&h.state, &ch, &mut session, "notes/local-stale.md".into()).await;

    assert_eq!(
        session.active_repo.as_deref(),
        Some(h.state.repo.local_repo_name())
    );
    assert_eq!(session.active_repo_id, Some(h.repo_id));
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    assert!(h.state.repo.get_docid("notes/local-stale.md")?.is_some());
    assert!(h.workspace_path("notes/local-stale.md").exists());
    Ok(())
}
