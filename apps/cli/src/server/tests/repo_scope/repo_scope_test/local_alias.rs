//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::support::build_state;
use crate::server::{
    repo_scope::{map_repo_scope_error, resolve_session_repo_and_sync},
    session::WsSession,
};
use deve_core::codec;
use deve_core::ledger::{
    RepoInfo, RepoManager, REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA,
    REPO_SCHEMA_VERSION_METADATA_KEY,
};
use deve_core::protocol::ServerErrorCode;

fn rewrite_local_metadata(
    repo: &RepoManager,
    repo_name: &str,
    info: RepoInfo,
) -> anyhow::Result<()> {
    let repo_id = uuid::Uuid::parse_str(repo_name)?;
    let db = repo.lease_local_authority(repo_id)?;
    let txn = db.db().begin_write()?;
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
    // A session that presents a stale local display alias ("legacy-test") which no
    // longer matches the repo's canonical stem or metadata name must fail closed
    // rather than silently recover the repo from the still-bound canonical UUID.
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
        &test_id.to_string(),
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
        &test_id.to_string(),
        RepoInfo {
            uuid: test_id,
            name: test_id.to_string(),
            url: None,
        },
    )?;

    let mut session = WsSession::new();
    session.switch_repo(test_id.to_string(), Some(test_id));
    let err =
        resolve_session_repo_and_sync(&state, &mut session).expect_err("corrupted repo must fail");
    assert!(err.to_string().contains(&format!(
        "Broken local repo {test_id} while validating catalog: repository URL missing"
    )));
    let mapped = map_repo_scope_error(anyhow::anyhow!(err.to_string()));
    assert_eq!(mapped.code, ServerErrorCode::StoragePersistFailed);
    assert_eq!(
        session.active_repo.as_deref(),
        Some(test_id.to_string().as_str())
    );
    assert_eq!(session.active_repo_id, Some(test_id));
    Ok(())
}
