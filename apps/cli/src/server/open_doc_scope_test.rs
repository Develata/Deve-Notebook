//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime

use super::handlers::document::{handle_open_doc, handle_request_history};
use super::{
    docs_test_support::channel as unicast_channel,
    document_local_scope_test_support::{
        assert_protocol_error, browser_repo_session, delete_doc, repo_session, seed_doc,
    },
    document_remote_scope_state_test_support::build_state,
};
use deve_core::protocol::ServerErrorCode;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_doc_on_wrong_repo_returns_error_without_empty_snapshot() -> anyhow::Result<()> {
    let (_dir, state, test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "default", "hello")?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_repo_session("test", test_repo_id, 7);

    handle_open_doc(&state, &ch, &mut session, doc_id, 7).await;

    assert_protocol_error(&mut uni_rx, None, Some(7), "must not send empty snapshot").await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_deleted_doc_returns_error_without_snapshot() -> anyhow::Result<()> {
    let (_dir, state, _test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "default", "hello")?;
    let default_id = delete_doc(&state, doc_id)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = repo_session("default", default_id);

    handle_open_doc(&state, &ch, &mut session, doc_id, 8).await;

    assert_protocol_error(
        &mut uni_rx,
        Some(ServerErrorCode::DocNotFound),
        None,
        "must not send deleted snapshot",
    )
    .await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_history_on_wrong_repo_returns_error_without_history() -> anyhow::Result<()> {
    let (_dir, state, test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "default", "hello")?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_repo_session("test", test_repo_id, 9);

    handle_request_history(&state, &ch, &mut session, doc_id, 9).await;

    assert_protocol_error(&mut uni_rx, None, Some(9), "must not send empty history").await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_history_on_deleted_doc_returns_error_without_history() -> anyhow::Result<()> {
    let (_dir, state, _test_repo_id) = build_state()?;
    let doc_id = seed_doc(&state, "default", "hello")?;
    let default_id = delete_doc(&state, doc_id)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = repo_session("default", default_id);

    handle_request_history(&state, &ch, &mut session, doc_id, 10).await;

    assert_protocol_error(
        &mut uni_rx,
        Some(ServerErrorCode::DocNotFound),
        None,
        "must not send deleted history",
    )
    .await;
    Ok(())
}
