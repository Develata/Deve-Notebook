//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime

use super::handlers::document::{handle_open_doc, handle_request_history};
use super::handlers::listing::handle_list_docs;
use super::{
    document_remote_scope_test_support::{
        assert_doc_list, assert_history, assert_snapshot, remote_browser_session, seed_shadow_doc,
    },
    document_remote_scope_state_test_support::build_state,
    docs_test_support::channel as unicast_channel,
};
use deve_core::models::PeerId;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_doc_on_remote_branch_uses_shadow_repo_without_locked_db() -> anyhow::Result<()> {
    let (_dir, state, test_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let doc_id = seed_shadow_doc(&state, &peer_id, test_repo_id)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = remote_browser_session(&peer_id, test_repo_id, 13);

    handle_open_doc(&state, &ch, &mut session, doc_id, 9).await;

    assert_snapshot(&mut uni_rx, test_repo_id, doc_id, 13).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_history_on_remote_branch_uses_shadow_repo_without_locked_db() -> anyhow::Result<()>
{
    let (_dir, state, test_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let doc_id = seed_shadow_doc(&state, &peer_id, test_repo_id)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = remote_browser_session(&peer_id, test_repo_id, 17);

    handle_request_history(&state, &ch, &mut session, doc_id, 11).await;

    assert_history(&mut uni_rx, test_repo_id, 17).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_docs_on_remote_branch_uses_shadow_repo_without_locked_db() -> anyhow::Result<()> {
    let (_dir, state, test_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let _ = seed_shadow_doc(&state, &peer_id, test_repo_id)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = remote_browser_session(&peer_id, test_repo_id, 19);

    handle_list_docs(&state, &ch, &mut session, None, None).await;

    assert_doc_list(&mut uni_rx, test_repo_id).await;
    Ok(())
}
