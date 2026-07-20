//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_target_branch_lacks_current_repo_match(
) -> anyhow::Result<()> {
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, default_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger_dir,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let default_info = repo
        .get_local_repo_info_by_id(default_id)?
        .expect("default info");
    let notes_id = crate::server::catalog_repo_support::catalog_additional_repo(
        &repo,
        &ledger_dir,
        "notes",
        &projection_base,
        10,
        Some("urn:notes"),
    )?;
    let notes_info = repo
        .get_local_repo_info_by_id(notes_id)?
        .expect("notes info");
    let ghost_id = crate::server::catalog_repo_support::catalog_additional_repo(
        &repo,
        &ledger_dir,
        "ghost",
        &projection_base,
        10,
        Some("urn:ghost"),
    )?;
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(&peer_id, &default_info)?;
    repo.ensure_shadow_repo_info(&peer_id, &notes_info)?;
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(0);
    session.switch_repo(ghost_id.to_string(), Some(ghost_id));

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
                    .is_some_and(|detail| detail.contains("Repository UUID not resolved")),
                "unexpected detail: {:?}",
                error.detail
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert_eq!(
        session.active_repo.as_deref(),
        Some(ghost_id.to_string().as_str())
    );
    assert_eq!(session.active_repo_id, Some(ghost_id));
    Ok(())
}
