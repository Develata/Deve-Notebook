use crate::server::repo_scope::{ResolvedRepo, resolve_local_counterpart_repo};
use crate::server::{AppState, channel::DualChannel};
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::DOCID_TO_PATH;
use deve_core::models::DocId;
use std::sync::Arc;

use super::errors;

pub(super) fn resolve_doc_path(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_name: &str,
    doc_id: DocId,
    scope_nonce: Option<u64>,
) -> Option<String> {
    match state
        .repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)
    {
        Ok(Some(meta)) => Some(meta.path),
        Err(e) => {
            errors::classified_failure(
                ch,
                format!("Failed to resolve merged doc path: {}", e),
                scope_nonce,
            );
            None
        }
        Ok(None) => {
            match legacy_doc_path(state.repo.as_ref(), repo_name, doc_id) {
                Ok(Some(path)) => errors::classified_failure(
                    ch,
                    format!(
                        "Tracked document projection missing for legacy-mapped doc: {}",
                        path
                    ),
                    scope_nonce,
                ),
                Ok(None) => errors::storage_not_found(
                    ch,
                    "Doc path not found for merged document",
                    scope_nonce,
                ),
                Err(e) => errors::classified_failure(
                    ch,
                    format!("Failed to resolve merged doc path: {}", e),
                    scope_nonce,
                ),
            }
            None
        }
    }
}

fn legacy_doc_path(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
) -> anyhow::Result<Option<String>> {
    repo.run_on_local_repo(repo_name, |db| {
        let read = db.begin_read()?;
        let table = read.open_table(DOCID_TO_PATH)?;
        Ok(table
            .get(doc_id.as_u128())?
            .map(|path| path.value().to_string()))
    })
}

pub(super) fn resolve_local_merge_scope(
    state: &Arc<AppState>,
    scope: ResolvedRepo,
    ch: &DualChannel,
    scope_nonce: Option<u64>,
) -> Option<ResolvedRepo> {
    match resolve_local_counterpart_repo(state, &scope) {
        Ok(Some(local_scope)) => Some(local_scope),
        Ok(None) => {
            errors::storage_not_found(
                ch,
                "No local repository matched the active remote repository",
                scope_nonce,
            );
            None
        }
        Err(err) => {
            errors::classified_failure(
                ch,
                format!("Failed to resolve local merge scope: {}", err),
                scope_nonce,
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::AppState;
    use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
    use deve_core::models::PeerId;
    use deve_core::protocol::{ServerErrorCode, ServerMessage};
    use tempfile::tempdir;
    use tokio::sync::broadcast;

    #[test]
    fn resolves_local_merge_scope_from_remote_repo_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        let vault = dir.path().join("vault");
        let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)
            .expect("init repo");
        repo.set_vault_root(&vault);
        let repo = Arc::new(repo);
        let info = repo
            .get_repo_info_for(None, Some(repo.local_repo_name()))
            .expect("repo info")
            .expect("repo info exists");
        let state = Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx: broadcast::channel(8).0,
            plugins: vec![],
            sync_engine: Arc::new(deve_core::sync::repo_scoped::RepoScopedSyncEngine::new(
                PeerId::new("local"),
                repo.clone(),
                deve_core::config::SyncMode::Auto,
            )),
            tree_manager: Arc::new(crate::server::tree_state::RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key: Arc::new(deve_core::security::IdentityKeyPair::generate()),
        });
        let ch = crate::server::channel::DualChannel::new(
            broadcast::channel(8).0,
            crate::server::ws::send::new_unicast_channel().0,
        );

        let resolved = resolve_local_merge_scope(
            &state,
            ResolvedRepo {
                repo_id: info.uuid,
                repo_name: "shadow-repo".into(),
                branch: Some(PeerId::new("remote-peer")),
            },
            &ch,
            None,
        )
        .expect("resolved scope");

        assert_eq!(resolved.repo_name, repo.local_repo_name());
        assert_eq!(resolved.repo_id, info.uuid);
        assert!(resolved.branch.is_none());
    }

    #[test]
    fn resolve_doc_path_fails_closed_on_legacy_only_projection() {
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault");
        let mut repo = RepoManager::init(
            dir.path().join("ledger"),
            10,
            Some("default"),
            Some("urn:default"),
        )
        .expect("init repo");
        repo.set_vault_root(&vault);
        let info = repo.get_repo_info().expect("repo info").expect("present");
        let doc_id = DocId(uuid::Uuid::new_v4());
        repo.run_on_local_repo("default", |db| {
            let write = db.begin_write()?;
            write.open_table(REPO_METADATA)?.insert(
                &0,
                bincode::serialize(&RepoInfo {
                    uuid: info.uuid,
                    name: "default".into(),
                    url: Some("urn:default".into()),
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
        let ch = crate::server::channel::DualChannel::new(broadcast::channel(8).0, unicast_tx);
        let state = Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx: broadcast::channel(8).0,
            plugins: vec![],
            sync_engine: Arc::new(deve_core::sync::repo_scoped::RepoScopedSyncEngine::new(
                PeerId::new("local"),
                repo.clone(),
                deve_core::config::SyncMode::Auto,
            )),
            tree_manager: Arc::new(crate::server::tree_state::RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key: Arc::new(deve_core::security::IdentityKeyPair::generate()),
        });

        assert!(resolve_doc_path(&state, &ch, "default", doc_id, Some(7)).is_none());

        match unicast_rx.blocking_recv() {
            Some(ServerMessage::ProtocolError {
                error, scope_nonce, ..
            }) => {
                assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
                assert_eq!(scope_nonce, Some(7));
                assert!(
                    error.detail.as_deref().is_some_and(
                        |detail| detail.contains("Tracked document projection missing")
                    )
                );
            }
            other => panic!("expected scoped ProtocolError, got {:?}", other),
        }
    }
}
