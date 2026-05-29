//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::AppState;
use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::session::WsSession;
use crate::server::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use std::sync::Arc;
use tempfile::{TempDir, tempdir};

type Harness = (TempDir, Arc<AppState>, RepoInfo, PeerId);

fn state_with_shadow(create_notes: bool) -> anyhow::Result<Harness> {
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger_dir, 10, Some("default"), Some("urn:default"))?;
    let default_info = repo.get_repo_info()?.expect("default repo info");
    if create_notes {
        RepoManager::init(&ledger_dir, 10, Some("notes"), Some("urn:notes"))?;
    }
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(&peer_id, &default_info)?;
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    Ok((dir, state, default_info, peer_id))
}

async fn assert_switch_rejects(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    session: &mut WsSession,
    switch_nonce: u64,
) -> anyhow::Result<()> {
    let (ch, mut uni_rx) = unicast_channel(state);
    handle_switch_branch(
        state,
        &ch,
        session,
        Some(peer_id.to_string()),
        Some(switch_nonce),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce: actual,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(actual, Some(switch_nonce));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert_eq!(
        (session.active_repo.as_deref(), session.active_repo_id),
        (None, None)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_current_local_scope_selector_is_stale()
-> anyhow::Result<()> {
    let (_dir, state, default_info, peer_id) = state_with_shadow(true)?;
    let mut session = browser_session(54);
    session.switch_repo("notes".into(), Some(default_info.uuid));

    assert_switch_rejects(&state, &peer_id, &mut session, 55).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_current_local_scope_hint_is_raw_uuid() -> anyhow::Result<()>
{
    let (_dir, state, default_info, peer_id) = state_with_shadow(false)?;
    let mut session = browser_session(56);
    session.switch_repo(default_info.uuid.to_string(), None);

    assert_switch_rejects(&state, &peer_id, &mut session, 57).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_current_local_scope_hint_is_stale_name()
-> anyhow::Result<()> {
    let (_dir, state, _default_info, peer_id) = state_with_shadow(false)?;
    let mut session = browser_session(57);
    session.switch_repo("stale-default".into(), None);

    assert_switch_rejects(&state, &peer_id, &mut session, 58).await
}
