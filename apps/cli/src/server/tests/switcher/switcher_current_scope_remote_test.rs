//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::handlers::switcher::handle_switch_branch;
use super::switcher_test_support::{app_state, browser_session, unicast_channel};
use super::AppState;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use std::sync::Arc;
use tempfile::{tempdir, TempDir};

fn state_with_remote(url: Option<&str>) -> anyhow::Result<(TempDir, Arc<AppState>, PeerId)> {
    let dir = tempdir()?;
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(
        dir.path().join("ledger"),
        10,
        Some("default"),
        Some("urn:default"),
    )?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: url.map(str::to_owned),
        },
    )?;
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    Ok((dir, state, peer_id))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_current_remote_scope_selector_is_stale(
) -> anyhow::Result<()> {
    let (_dir, state, peer_id) = state_with_remote(Some("urn:wiki-a"))?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(72);
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("stale-wiki".into(), None);

    handle_switch_branch(&state, &ch, &mut session, None, Some(73)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(error
                .detail
                .as_deref()
                .is_some_and(|detail| detail.starts_with("stale remote scope:")));
            assert_eq!(switch_nonce, Some(73));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, Some(peer_id));
    assert_eq!(
        (session.active_repo.as_deref(), session.active_repo_id),
        (None, None)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_current_remote_scope_has_no_url() -> anyhow::Result<()> {
    let (_dir, state, peer_id) = state_with_remote(None)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(73);
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("wiki".into(), None);

    handle_switch_branch(&state, &ch, &mut session, None, Some(74)).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(error
                .detail
                .as_deref()
                .is_some_and(|detail| detail.starts_with("stale remote scope:")));
            assert_eq!(switch_nonce, Some(74));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, Some(peer_id));
    assert_eq!(session.active_repo.as_deref(), None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}
