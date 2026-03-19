use super::repo_scope::{map_repo_scope_error, resolve_session_repo_and_sync};
use super::{AppState, session::WsSession, tree_state::RepoTreeRegistry};
use crate::server::security;
use deve_core::config::SyncMode;
use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::broadcast;

#[test]
fn resolve_session_repo_fails_closed_on_stale_local_alias_drift() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let wiki = RepoManager::init(dir.path(), 10, Some("wiki"), Some("urn:wiki"))?;
    let wiki_info = wiki.get_repo_info()?.expect("wiki info");
    let wiki_db = repo.open_database(None, "wiki")?.db;
    let txn = wiki_db.begin_write()?;
    txn.open_table(REPO_METADATA)?.insert(
        &0,
        bincode::serialize(&RepoInfo {
            uuid: wiki_info.uuid,
            name: "legacy-wiki".into(),
            url: wiki_info.url.clone(),
        })?
        .as_slice(),
    )?;
    txn.commit()?;

    let repo = Arc::new(repo);
    let state = Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
        tx: broadcast::channel(16).0,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            PeerId::new("test-peer"),
            repo,
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_service: None,
        identity_key: security::load_or_generate_identity_key(&dir.path().join("host"))?,
    });

    let mut session = WsSession::new();
    session.switch_repo("legacy-wiki".into(), Some(wiki_info.uuid));
    let err = resolve_session_repo_and_sync(&state, &mut session).expect_err("must fail closed");
    let mapped = map_repo_scope_error(anyhow::anyhow!(err.to_string()));
    assert_eq!(
        mapped.code,
        deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
    );
    assert_eq!(session.active_repo.as_deref(), None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[test]
fn open_database_rejects_stale_local_alias_after_metadata_drift() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    let wiki = RepoManager::init(dir.path(), 10, Some("wiki"), Some("urn:wiki"))?;
    let wiki_info = wiki.get_repo_info()?.expect("wiki info");
    let wiki_db = repo.open_database(None, "wiki")?.db;
    let txn = wiki_db.begin_write()?;
    txn.open_table(REPO_METADATA)?.insert(
        &0,
        bincode::serialize(&RepoInfo {
            uuid: wiki_info.uuid,
            name: "legacy-wiki".into(),
            url: wiki_info.url.clone(),
        })?
        .as_slice(),
    )?;
    txn.commit()?;

    let err = match repo.open_database(None, "legacy-wiki") {
        Ok(_) => anyhow::bail!("stale local alias must fail closed"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("legacy-wiki"));
    Ok(())
}
