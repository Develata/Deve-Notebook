//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime

use super::handlers::document::{handle_open_doc, handle_request_history};
use super::{
    docs_test_support::channel as unicast_channel,
    document_bootstrap_test_support::{
        assert_bootstrapped_session, assert_history, assert_snapshot, assert_stale_binding_cleared,
        stale_local_binding_session,
    },
    document_local_scope_test_support::seed_doc,
    document_remote_scope_state_test_support::build_single_repo_state,
};
use super::session::WsSession;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_doc_without_repo_selection_bootstraps_single_repo() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_single_repo_state()?;
    let doc_id = seed_doc(&state, "default", "hello")?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = WsSession::new();

    handle_open_doc(&state, &ch, &mut session, doc_id, 1).await;

    assert_snapshot(&mut uni_rx, default_id, doc_id).await;
    assert_bootstrapped_session(&session, default_id);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_history_without_repo_selection_bootstraps_single_repo() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_single_repo_state()?;
    let doc_id = seed_doc(&state, "default", "hello")?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = WsSession::new();

    handle_request_history(&state, &ch, &mut session, doc_id, 2).await;

    assert_history(&mut uni_rx, default_id, doc_id).await;
    assert_bootstrapped_session(&session, default_id);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_doc_with_stale_local_binding_bootstraps_single_repo() -> anyhow::Result<()> {
    let (dir, state, default_id) = build_single_repo_state()?;
    let doc_id = seed_doc(&state, "default", "hello")?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = stale_local_binding_session(dir.path())?;

    handle_open_doc(&state, &ch, &mut session, doc_id, 3).await;

    assert_snapshot(&mut uni_rx, default_id, doc_id).await;
    assert_bootstrapped_session(&session, default_id);
    assert_stale_binding_cleared(&session);
    Ok(())
}
