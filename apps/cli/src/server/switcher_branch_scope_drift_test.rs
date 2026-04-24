use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_branch_scope_test_support::{
    app_state, browser_session, unicast_channel,
};
use deve_core::ledger::schema::REPO_METADATA;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_local_display_name_drift_matches_shadow_peer()
-> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let local = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:notes"))?;
    local.run_on_local_repo("notes", |db| {
        let read = db.begin_read()?;
        let table = read.open_table(REPO_METADATA)?;
        let raw = table.get(&0)?.expect("repo info");
        let mut info: RepoInfo = bincode::deserialize(raw.value())?;
        info.name = "peer-remote".into();
        drop(table);
        drop(read);
        let write = db.begin_write()?;
        {
            let mut table = write.open_table(REPO_METADATA)?;
            table.insert(&0, bincode::serialize(&info)?.as_slice())?;
        }
        write.commit()?;
        Ok(())
    })?;
    let local = local
        .get_repo_info_for(None, Some("notes"))?
        .expect("local repo info");
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(&peer_id, &local)?;
    let state = app_state(repo, vault, dir.path().join("host"))?;
    let (ch, mut uni_rx) = unicast_channel(&state);
    let mut session = browser_session(0);

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
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    Ok(())
}
