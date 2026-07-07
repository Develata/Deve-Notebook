//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::handlers::switcher::handle_switch_branch;
use super::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::codec;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_current_local_scope_hint_is_raw_uuid_string()
-> anyhow::Result<()> {
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger_dir, 10, Some("default"), Some("urn:default"))?;
    let local = RepoManager::init(&ledger_dir, 10, Some("notes"), Some("urn:notes"))?;
    let local_info = local.get_repo_info()?.expect("local notes info");
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let peer_id = PeerId::new("peer-remote");
    let remote_repo_id = uuid::Uuid::new_v4();
    repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: remote_repo_id,
            name: "shadow-notes".into(),
            url: Some("urn:notes".into()),
        },
    )?;
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(28);
    session.switch_repo(local_info.uuid.to_string(), None);

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(29),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(switch_nonce, Some(29));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert!(session.active_repo.is_none());
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_current_local_scope_metadata_is_broken()
-> anyhow::Result<()> {
    let dir = tempdir()?;
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(
        dir.path().join("ledger"),
        10,
        Some("default"),
        Some("urn:default"),
    )?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let local_info = repo.get_repo_info()?.expect("default repo info");
    let peer_id = PeerId::new("peer-remote");
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    state.repo.ensure_shadow_repo_info(&peer_id, &local_info)?;
    let db = state.repo.open_database(None, "default")?.db;
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = codec::encode(&REDB_SCHEMA_VERSION)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        table.insert(&REPO_INFO_METADATA_KEY, [0_u8, 1, 2, 3].as_slice())?;
    }
    txn.commit()?;

    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(40);
    session.switch_repo("default".into(), Some(local_info.uuid));

    handle_switch_branch(
        &state,
        &ch,
        &mut session,
        Some(peer_id.to_string()),
        Some(41),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(switch_nonce, Some(41));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    Ok(())
}
