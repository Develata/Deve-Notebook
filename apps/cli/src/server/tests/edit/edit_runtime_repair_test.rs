use super::{
    edit_message_test_support::{recv_ack, send_insert},
    edit_state_test_support::{edit_harness, seed_doc, unicast_channel, writer_browser_session},
};
use deve_core::ledger::schema::CLIENT_OP_INDEX;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_repairs_missing_client_op_index_for_secondary_local_repo() -> anyhow::Result<()> {
    let h = edit_harness(true)?;
    let test_repo_id = h.test_repo_id.expect("test repo id");
    let test_repo_name = h.test_repo_name.as_deref().expect("test repo name");
    let doc_id = seed_doc(&h.state, test_repo_name, "notes/a.md")?;
    h.state.repo.run_on_local_repo(test_repo_name, |db| {
        let write = db.begin_write()?;
        let _ = write.delete_table(CLIENT_OP_INDEX)?;
        write.commit()?;
        Ok(())
    })?;

    let (ch, mut uni_rx) = unicast_channel(&h.state);
    let mut session = writer_browser_session(test_repo_name, test_repo_id, 31);
    send_insert(&h.state, &ch, &mut session, doc_id, 0).await;

    let (scope_nonce, ack_doc, client_op_id) = recv_ack(&mut uni_rx).await;
    assert_eq!(scope_nonce, Some(31));
    assert_eq!(ack_doc, doc_id);
    assert_eq!(client_op_id, 9);
    Ok(())
}
