//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::ledger::RepoInfo;
use deve_core::models::PeerId;
use deve_core::protocol::{RepoReadiness, ServerMessage};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_uses_exact_repo_id_without_transporting_local_alias() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, _default_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger_dir,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let notes_id = crate::server::catalog_repo_support::catalog_additional_repo(
        &repo,
        &ledger_dir,
        "notes",
        &projection_base,
        10,
        Some("urn:notes"),
    )?;
    assert_eq!(
        repo.host_repo_alias_runtime().binding(notes_id)?.alias,
        "notes"
    );
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: notes_id,
            name: notes_id.to_string(),
            url: Some("urn:remote:independent".into()),
        },
    )?;
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(0);
    session.switch_repo(notes_id.to_string(), Some(notes_id));

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(1),
    )
    .await;

    let first = uni_rx.recv().await;
    assert!(
        matches!(
            first,
        Some(ServerMessage::BranchSwitched {
            success: true,
            switch_nonce: Some(1),
            ..
        })
        ),
        "expected successful BranchSwitched, got {first:?}"
    );
    match uni_rx.recv().await {
        Some(ServerMessage::RepoList {
            branch: Some(branch),
            repo_entries,
            ..
        }) => {
            assert_eq!(branch, peer_id.to_string());
            assert_eq!(repo_entries.len(), 1);
            assert_eq!(repo_entries[0].repo_id, notes_id);
            assert_eq!(repo_entries[0].display_alias, notes_id.to_string());
            assert_eq!(repo_entries[0].readiness, RepoReadiness::Readonly);
        }
        other => panic!("expected exact remote RepoList, got {other:?}"),
    }
    match uni_rx.recv().await {
        Some(ServerMessage::RepoSwitched {
            branch: Some(branch),
            repo_id,
            display_alias,
            switch_nonce: Some(1),
            ..
        }) => {
            assert_eq!(branch, peer_id.to_string());
            assert_eq!(repo_id, notes_id);
            assert_eq!(display_alias, notes_id.to_string());
        }
        other => panic!("expected exact remote RepoSwitched, got {other:?}"),
    }
    assert_eq!(session.active_branch.as_ref(), Some(&peer_id));
    assert_eq!(session.active_repo_id, Some(notes_id));
    Ok(())
}
