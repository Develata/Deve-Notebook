use super::{
    edit_message_test_support::{recv_edit_rejected, send_insert},
    edit_state_test_support::{
        edit_harness, seed_doc_with_content, unicast_channel, writer_browser_session,
    },
};
use deve_core::ledger::schema::CLIENT_OP_INDEX;
use deve_core::protocol::ServerErrorCode;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_rejects_doc_outside_active_repo_before_append() -> anyhow::Result<()> {
    let h = edit_harness(true)?;
    let test_repo_id = h.test_repo_id.expect("test repo id");
    let doc_id = seed_doc_with_content(&h.state, "default", "notes/a.md", "hello")?;
    let (ch, mut uni_rx) = unicast_channel(&h.state);
    let mut session = writer_browser_session("test", test_repo_id, 19);

    send_insert(&h.state, &ch, &mut session, doc_id, 5).await;

    let (scope_nonce, rejected_doc_id, client_op_id, error) =
        recv_edit_rejected(&mut uni_rx).await;
    assert_eq!(scope_nonce, 19);
    assert_eq!(rejected_doc_id, doc_id);
    assert_eq!(client_op_id, 9);
    assert_eq!(error.code, ServerErrorCode::DocNotFound);
    assert!(
        h.state
            .repo
            .find_client_op_in_local_repo("test", 7, 9)?
            .is_none(),
        "must not append orphan op into active repo"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_fails_closed_on_broken_client_op_index() -> anyhow::Result<()> {
    let h = edit_harness(true)?;
    let doc_id = seed_doc_with_content(&h.state, "default", "notes/a.md", "hello")?;
    let op_count_before = h.state.repo.get_local_ops(doc_id)?.len();
    h.state.repo.run_on_local_repo("default", |db| {
        let write = db.begin_write()?;
        {
            let mut client_ops = write.open_table(CLIENT_OP_INDEX)?;
            client_ops.insert((7, 9), 999)?;
        }
        write.commit()?;
        Ok(())
    })?;

    let (ch, mut uni_rx) = unicast_channel(&h.state);
    let mut session = writer_browser_session("default", h.default_repo_id, 23);
    send_insert(&h.state, &ch, &mut session, doc_id, 5).await;

    let (scope_nonce, rejected_doc_id, client_op_id, error) =
        recv_edit_rejected(&mut uni_rx).await;
    assert_eq!(scope_nonce, 23);
    assert_eq!(rejected_doc_id, doc_id);
    assert_eq!(client_op_id, 9);
    assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
    assert!(
        error
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("Broken client op index")),
        "unexpected detail: {:?}",
        error.detail
    );
    assert_eq!(h.state.repo.get_local_ops(doc_id)?.len(), op_count_before);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_rejects_degraded_local_projection_before_append() -> anyhow::Result<()> {
    let h = edit_harness(false)?;
    let doc_id = seed_doc_with_content(&h.state, "default", "notes/a.md", "hello")?;
    let op_count_before = h.state.repo.get_local_ops(doc_id)?.len();
    h.state
        .sync_manager
        .mark_projection_writeback_fault("default");
    let (ch, mut uni_rx) = unicast_channel(&h.state);
    let mut session = writer_browser_session("default", h.default_repo_id, 31);

    send_insert(&h.state, &ch, &mut session, doc_id, 5).await;

    let (scope_nonce, rejected_doc_id, client_op_id, error) =
        recv_edit_rejected(&mut uni_rx).await;
    assert_eq!(scope_nonce, 31);
    assert_eq!(rejected_doc_id, doc_id);
    assert_eq!(client_op_id, 9);
    assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
    assert_eq!(h.state.repo.get_local_ops(doc_id)?.len(), op_count_before);
    Ok(())
}
