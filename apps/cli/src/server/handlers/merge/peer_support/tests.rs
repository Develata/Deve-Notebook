//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Tests for peer merge support helpers.

use super::*;
use crate::server::tree_state::RepoTreeRegistry;
use deve_core::codec;
use deve_core::ledger::schema::DOCID_TO_PATH;
use deve_core::ledger::{REPO_INFO_METADATA_KEY, REPO_METADATA, RepoInfo};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tempfile::tempdir;
use tokio::sync::broadcast;

fn app_state(repo: Arc<RepoManager>) -> Arc<AppState> {
    Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(
            deve_core::sync::SyncManager::new_checked(repo.clone())
                .expect("projection locator must be valid"),
        ),
        tx: broadcast::channel(8).0,
        plugins: vec![],
        sync_engine: Arc::new(deve_core::sync::repo_scoped::RepoScopedSyncEngine::new(
            PeerId::new("local"),
            repo,
            deve_core::config::SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_available: false,
        identity_key: Arc::new(deve_core::security::IdentityKeyPair::generate()),
    })
}

#[test]
fn resolve_doc_path_fails_closed_on_legacy_only_projection() {
    let dir = tempdir().expect("tempdir");
    let projection_base = dir.path().join("notes");
    let repo =
        crate::test_support::init_cataloged_repo(&dir.path().join("ledger"), &projection_base, 10)
            .expect("init repo")
            .repo;
    let info = repo.get_repo_info().expect("repo info").expect("present");
    let repo_name = info.uuid.to_string();
    let doc_id = DocId(uuid::Uuid::new_v4());

    repo.run_on_local_repo(&repo_name, |db| {
        let write = db.begin_write()?;
        write.open_table(REPO_METADATA)?.insert(
            &REPO_INFO_METADATA_KEY,
            codec::encode(&RepoInfo {
                uuid: info.uuid,
                name: repo_name.clone(),
                url: info.url.clone(),
            })?
            .as_slice(),
        )?;
        write
            .open_table(DOCID_TO_PATH)?
            .insert(doc_id.as_u128(), "notes/legacy.md")?;
        write.commit()?;
        Ok(())
    })
    .expect("seed legacy path");

    let repo = Arc::new(repo);
    let (unicast_tx, mut unicast_rx) = crate::server::ws::send::new_unicast_channel();
    let ch = DualChannel::new(broadcast::channel(8).0, unicast_tx);
    let state = app_state(repo);

    assert!(resolve_doc_path(&state, &ch, &repo_name, doc_id, Some(7)).is_none());

    match unicast_rx.blocking_recv() {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(scope_nonce, Some(7));
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("Tracked document projection missing"))
            );
        }
        other => panic!("expected scoped ProtocolError, got {:?}", other),
    }
}
