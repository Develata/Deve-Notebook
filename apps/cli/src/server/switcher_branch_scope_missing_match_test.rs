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
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let default_info = repo.get_repo_info()?.expect("default info");
    let mut notes_repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:notes"))?;
    notes_repo.set_vault_root(&vault);
    let notes_info = notes_repo.get_repo_info()?.expect("notes info");
    let mut ghost_repo = RepoManager::init(dir.path(), 10, Some("ghost"), Some("urn:ghost"))?;
    ghost_repo.set_vault_root(&vault);
    let state = app_state(repo, vault, dir.path().join("host"))?;
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
