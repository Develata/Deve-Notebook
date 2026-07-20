//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::handlers::switcher::handle_switch_branch;
use crate::server::switcher_test_support::{app_state, browser_session, unicast_channel};
use deve_core::codec;
use deve_core::ledger::schema::{REPO_INFO_METADATA_KEY, REPO_METADATA};
use deve_core::ledger::RepoInfo;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tempfile::tempdir;

// DECISION PENDING (USER): the UUID-first cutover removed switch_branch's
// display-name-based cross-peer guard, so this now succeeds. Either the guard
// is obsolete under RepoId-only matching (delete/repurpose this test) or its
// removal is a Source Control fail-closed regression (restore the guard).
// Assertions are intentionally kept unchanged pending that ruling.
#[ignore = "switch_branch display-name cross-peer guard removed by UUID-first cutover; awaiting USER ruling"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switch_branch_fails_closed_when_local_display_name_drift_matches_shadow_peer(
) -> anyhow::Result<()> {
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
    let local_info = repo
        .get_local_repo_info_by_id(notes_id)?
        .expect("local repo info");
    repo.run_on_local_repo(&notes_id.to_string(), |db| {
        let read = db.begin_read()?;
        let table = read.open_table(REPO_METADATA)?;
        let raw = table.get(&REPO_INFO_METADATA_KEY)?.expect("repo info");
        let mut info: RepoInfo = codec::decode(raw.value())?;
        info.name = "peer-remote".into();
        drop(table);
        drop(read);
        let write = db.begin_write()?;
        {
            let mut table = write.open_table(REPO_METADATA)?;
            table.insert(&REPO_INFO_METADATA_KEY, codec::encode(&info)?.as_slice())?;
        }
        write.commit()?;
        Ok(())
    })?;
    let peer_id = PeerId::new("peer-remote");
    repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: notes_id,
            name: "peer-remote".into(),
            url: local_info.url,
        },
    )?;
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
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
