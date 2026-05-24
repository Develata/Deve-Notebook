//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_target_branch_lacks_current_repo_match()
-> anyhow::Result<()> {
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger_dir, 10, Some("default"), Some("urn:default"))?;
    let default_info = repo.get_repo_info()?.expect("default info");
    let notes_repo = RepoManager::init(&ledger_dir, 10, Some("notes"), Some("urn:notes"))?;
    let notes_info = notes_repo.get_repo_info()?.expect("notes info");
    RepoManager::init(&ledger_dir, 10, Some("ghost"), Some("urn:ghost"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    let peer_id = PeerId::new("peer-remote");
    state
        .repo
        .ensure_shadow_repo_info(&peer_id, &default_info)?;
    state.repo.ensure_shadow_repo_info(&peer_id, &notes_info)?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(0);
    session.switch_repo("ghost".into(), None);

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(1),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("repository selector not resolved")),
                "unexpected detail: {:?}",
                error.detail
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert_eq!(session.active_repo.as_deref(), Some("ghost"));
    assert_eq!(session.active_repo_id, None);
    Ok(())
}
