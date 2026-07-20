use super::{
    edit_message_test_support::{recv_edit_rejected, send_insert, send_insert_with_scope},
    edit_state_test_support::{
        edit_harness, seed_doc_with_content, unicast_channel, writer_browser_session,
    },
};
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_clears_stale_remote_readonly_binding_before_checks() -> anyhow::Result<()> {
    let h = edit_harness(false)?;
    let doc_id = seed_doc_with_content(&h.state, &h.default_repo_name, "notes/a.md", "hello")?;
    let stale_db = Arc::new(redb::Database::create(
        h.dir.path().join("stale-remote.redb"),
    )?);
    let (ch, mut uni_rx) = unicast_channel(&h.state);
    let mut session = writer_browser_session(&h.default_repo_name, h.default_repo_id, 29);
    session.set_authenticated(PeerId::new("writer"));
    session.bind_repo(h.default_repo_id);
    session.set_active_db(DatabaseHandle {
        db: stale_db,
        readonly: true,
        branch: Some(PeerId::new("remote")),
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: "shadow".into(),
    });

    send_insert(&h.state, &ch, &mut session, doc_id, 5).await;

    let (scope_nonce, rejected_doc_id, client_op_id, error) = recv_edit_rejected(&mut uni_rx).await;
    assert_eq!(scope_nonce, 29);
    assert_eq!(rejected_doc_id, doc_id);
    assert_eq!(client_op_id, 9);
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.writer_identity.is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_rejects_stale_message_scope_at_writer_gate() -> anyhow::Result<()> {
    let h = edit_harness(false)?;
    let doc_id = seed_doc_with_content(&h.state, &h.default_repo_name, "notes/a.md", "hello")?;
    let op_count_before = h.state.repo.get_local_ops(doc_id)?.len();
    let (ch, mut uni_rx) = unicast_channel(&h.state);
    let mut session = writer_browser_session(&h.default_repo_name, h.default_repo_id, 42);

    send_insert_with_scope(&h.state, &ch, &mut session, doc_id, 5, Some(41)).await;

    let (scope_nonce, rejected_doc_id, client_op_id, error) = recv_edit_rejected(&mut uni_rx).await;
    assert_eq!(scope_nonce, 41);
    assert_eq!(rejected_doc_id, doc_id);
    assert_eq!(client_op_id, 9);
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert_eq!(h.state.repo.get_local_ops(doc_id)?.len(), op_count_before);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_edit_reject_uses_current_scope_when_request_scope_missing() -> anyhow::Result<()> {
    let h = edit_harness(false)?;
    let doc_id = seed_doc_with_content(&h.state, &h.default_repo_name, "notes/a.md", "hello")?;
    let op_count_before = h.state.repo.get_local_ops(doc_id)?.len();
    let (ch, mut uni_rx) = unicast_channel(&h.state);
    let mut session = writer_browser_session(&h.default_repo_name, h.default_repo_id, 42);

    send_insert_with_scope(&h.state, &ch, &mut session, doc_id, 5, None).await;

    let (scope_nonce, rejected_doc_id, client_op_id, error) = recv_edit_rejected(&mut uni_rx).await;
    assert_eq!(scope_nonce, 42);
    assert_eq!(rejected_doc_id, doc_id);
    assert_eq!(client_op_id, 9);
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert_eq!(h.state.repo.get_local_ops(doc_id)?.len(), op_count_before);
    Ok(())
}
