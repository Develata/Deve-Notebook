//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::support::build_state;
use crate::server::{
    repo_scope::{map_repo_scope_error, resolve_session_repo, resolve_session_repo_and_sync},
    session::WsSession,
};
use deve_core::protocol::ServerErrorCode;

#[test]
fn resolve_session_repo_preserves_missing_local_catalog_failure() -> anyhow::Result<()> {
    let (dir, state, _default_id, _test_id) = build_state()?;
    std::fs::remove_dir_all(dir.path().join("ledger").join("local"))?;

    let mut session = WsSession::new();
    session.switch_repo("test".into(), None);
    let err =
        resolve_session_repo(&state, &session).expect_err("missing local catalog must fail closed");

    let detail = err.to_string();
    assert!(
        detail.contains("local repo directory missing"),
        "unexpected error detail: {detail}"
    );
    assert_eq!(
        map_repo_scope_error(anyhow::anyhow!(detail)).code,
        ServerErrorCode::StoragePersistFailed
    );
    Ok(())
}

#[test]
fn resolve_session_repo_and_sync_clears_missing_remote_branch_scope() -> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), None);

    let err =
        resolve_session_repo_and_sync(&state, &mut session).expect_err("missing shadow must fail");

    assert!(err.to_string().contains("Remote branch not available:"));
    assert!(session.active_branch.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[test]
fn resolve_session_repo_preserves_missing_remote_catalog_failure() -> anyhow::Result<()> {
    let (dir, state, _default_id, _test_id) = build_state()?;
    std::fs::remove_dir_all(dir.path().join("ledger").join("remotes"))?;

    let mut session = WsSession::new();
    session.switch_branch(Some("peer-a".into()));
    let err = resolve_session_repo(&state, &session)
        .expect_err("missing remote catalog must fail closed");

    let detail = err.to_string();
    assert!(
        detail.contains("Broken remote repo catalog"),
        "unexpected error detail: {detail}"
    );
    assert_eq!(
        map_repo_scope_error(anyhow::anyhow!(detail)).code,
        ServerErrorCode::StoragePersistFailed
    );
    Ok(())
}
