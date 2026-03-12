use crate::server::repo_scope::{ResolvedRepo, resolve_local_counterpart_repo};
use crate::server::{AppState, channel::DualChannel};
use deve_core::models::DocId;
use std::sync::Arc;

use super::errors;

pub(super) fn resolve_doc_path(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_name: &str,
    doc_id: DocId,
) -> Option<String> {
    match state
        .repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)
    {
        Ok(Some(meta)) => Some(meta.path),
        Ok(None) => {
            errors::storage_not_found(ch, "Doc path not found for merged document");
            None
        }
        Err(e) => {
            errors::request_failed(ch, format!("Failed to resolve merged doc path: {}", e));
            None
        }
    }
}

pub(super) fn resolve_local_merge_scope(
    state: &Arc<AppState>,
    scope: ResolvedRepo,
    ch: &DualChannel,
) -> Option<ResolvedRepo> {
    match resolve_local_counterpart_repo(state, &scope) {
        Ok(Some(local_scope)) => Some(local_scope),
        Ok(None) => {
            errors::storage_not_found(
                ch,
                "No local repository matched the active remote repository",
            );
            None
        }
        Err(err) => {
            errors::request_failed(ch, format!("Failed to resolve local merge scope: {}", err));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::AppState;
    use deve_core::ledger::RepoManager;
    use deve_core::models::PeerId;
    use tokio::sync::broadcast;

    #[test]
    fn resolves_local_merge_scope_from_remote_repo_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = Arc::new(
            RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo"),
        );
        let info = repo
            .get_repo_info_for(None, Some(repo.local_repo_name()))
            .expect("repo info")
            .expect("repo info exists");
        let state = Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(
                repo.clone(),
                dir.path().join("vault"),
            )),
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
        )
        .expect("resolved scope");

        assert_eq!(resolved.repo_name, repo.local_repo_name());
        assert_eq!(resolved.repo_id, info.uuid);
        assert!(resolved.branch.is_none());
    }
}
