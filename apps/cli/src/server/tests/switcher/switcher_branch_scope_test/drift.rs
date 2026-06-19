//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::ledger::schema::{REPO_INFO_METADATA_KEY, REPO_METADATA};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_local_display_name_drift_matches_shadow_peer()
-> anyhow::Result<()> {
    let dir = tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger_dir, 10, Some("default"), Some("urn:default"))?;
    let local = RepoManager::init(&ledger_dir, 10, Some("notes"), Some("urn:notes"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let local_info = local.get_repo_info()?.expect("local repo info");
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    local.run_on_local_repo("notes", |db| {
        let read = db.begin_read()?;
        let table = read.open_table(REPO_METADATA)?;
        let raw = table.get(&REPO_INFO_METADATA_KEY)?.expect("repo info");
        let mut info: RepoInfo = bincode::deserialize(raw.value())?;
        info.name = "peer-remote".into();
        drop(table);
        drop(read);
        let write = db.begin_write()?;
        {
            let mut table = write.open_table(REPO_METADATA)?;
            table.insert(
                &REPO_INFO_METADATA_KEY,
                bincode::serialize(&info)?.as_slice(),
            )?;
        }
        write.commit()?;
        Ok(())
    })?;
    let peer_id = PeerId::new("peer-remote");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: local_info.uuid,
            name: "peer-remote".into(),
            url: local_info.url,
        },
    )?;
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
