use crate::server::channel::DualChannel;
use crate::server::repo_scope::{
    map_repo_scope_error, resolve_local_counterpart_repo, resolve_session_repo,
};
use crate::server::{AppState, session::WsSession};
use deve_core::models::RepoId;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

use super::errors;

pub(super) fn resolve_read_repo_id(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
) -> Option<RepoId> {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_protocol_error(map_repo_scope_error(err));
            return None;
        }
    };
    if scope.branch.is_none() {
        return Some(scope.repo_id);
    }
    match resolve_local_counterpart_repo(state, &scope) {
        Ok(Some(local_scope)) => Some(local_scope.repo_id),
        Ok(None) => {
            errors::storage_not_found(ch, "No local repository matched the active remote branch");
            None
        }
        Err(err) => {
            ch.send_protocol_error(map_repo_scope_error(err));
            None
        }
    }
}

pub(super) fn resolve_write_repo_id(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
) -> Option<RepoId> {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_protocol_error(map_repo_scope_error(err));
            return None;
        }
    };
    if scope.branch.is_some() {
        ch.send_protocol_error(ServerError::new(ServerErrorCode::ScRemoteBranchReadonly));
        return None;
    }
    Some(scope.repo_id)
}

#[cfg(test)]
mod tests {
    use super::resolve_read_repo_id;
    use crate::server::{AppState, channel::DualChannel, tree_state::RepoTreeRegistry};
    use deve_core::config::SyncMode;
    use deve_core::ledger::RepoManager;
    use deve_core::models::PeerId;
    use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::sync::broadcast;

    #[test]
    fn read_repo_id_uses_active_local_repo_without_sync_binding() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let vault = dir.path().join("vault");
        let mut repo = RepoManager::init(
            dir.path().join("ledger"),
            10,
            Some("default"),
            Some("urn:default"),
        )?;
        repo.set_vault_root(&vault);
        let mut test_repo = RepoManager::init(dir.path().join("ledger"), 10, Some("test"), None)?;
        test_repo.set_vault_root(&vault);
        let test_id = test_repo.get_repo_info()?.expect("test info").uuid;
        let repo = Arc::new(repo);
        let (tx, _rx) = broadcast::channel(16);
        let state = Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("local"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key: Arc::new(deve_core::security::IdentityKeyPair::generate()),
        });
        let ch = DualChannel::new(
            broadcast::channel(8).0,
            crate::server::ws::send::new_unicast_channel().0,
        );
        let mut session = crate::server::session::WsSession::new();
        session.switch_repo("test".into(), Some(test_id));

        assert_eq!(resolve_read_repo_id(&state, &ch, &session), Some(test_id));
        assert_eq!(session.bound_repo_id, None);
        Ok(())
    }
}
