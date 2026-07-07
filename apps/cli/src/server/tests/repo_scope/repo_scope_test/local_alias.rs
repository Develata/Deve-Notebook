//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::support::build_state;
use crate::server::{
    repo_scope::{map_repo_scope_error, resolve_session_repo_and_sync},
    session::WsSession,
};
use deve_core::codec;
use deve_core::ledger::{
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
    RepoInfo, RepoManager,
};
use deve_core::protocol::ServerErrorCode;

fn rewrite_local_metadata(
    repo: &RepoManager,
    repo_name: &str,
    info: RepoInfo,
) -> anyhow::Result<()> {
    let db = repo.open_database(None, repo_name)?.db;
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = codec::encode(&REDB_SCHEMA_VERSION)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        let bytes = codec::encode(&info)?;
        table.insert(&REPO_INFO_METADATA_KEY, bytes.as_slice())?;
    }
    txn.commit()?;
    Ok(())
}

#[test]
fn resolve_session_repo_fails_closed_on_stale_local_alias_drift() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    rewrite_local_metadata(
        state.repo.as_ref(),
        "test",
        RepoInfo {
            uuid: test_id,
            name: "legacy-test".into(),
            url: Some("urn:test".into()),
        },
    )?;

    let mut session = WsSession::new();
    session.switch_repo("legacy-test".into(), Some(test_id));
    let err = resolve_session_repo_and_sync(&state, &mut session).expect_err("must fail closed");
    let mapped = map_repo_scope_error(anyhow::anyhow!(err.to_string()));
    assert_eq!(mapped.code, ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[test]
fn open_database_rejects_stale_local_alias_after_metadata_drift() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    rewrite_local_metadata(
        state.repo.as_ref(),
        "test",
        RepoInfo {
            uuid: test_id,
            name: "legacy-test".into(),
            url: Some("urn:test".into()),
        },
    )?;

    let err = match state.repo.open_database(None, "legacy-test") {
        Ok(_) => anyhow::bail!("stale local alias must fail closed"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("legacy-test"));
    Ok(())
}

#[test]
fn resolve_session_repo_preserves_local_catalog_corruption_for_exact_selector() -> anyhow::Result<()>
{
    let (_dir, state, _default_id, test_id) = build_state()?;
    rewrite_local_metadata(
        state.repo.as_ref(),
        "test",
        RepoInfo {
            uuid: test_id,
            name: "test".into(),
            url: None,
        },
    )?;

    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(test_id));
    let err =
        resolve_session_repo_and_sync(&state, &mut session).expect_err("corrupted repo must fail");
    assert!(
        err.to_string()
            .contains("Broken local repo test while validating catalog: repository URL missing")
    );
    let mapped = map_repo_scope_error(anyhow::anyhow!(err.to_string()));
    assert_eq!(mapped.code, ServerErrorCode::StoragePersistFailed);
    assert_eq!(session.active_repo.as_deref(), Some("test"));
    assert_eq!(session.active_repo_id, Some(test_id));
    Ok(())
}
