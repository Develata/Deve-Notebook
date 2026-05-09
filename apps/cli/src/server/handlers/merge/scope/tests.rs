//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Merge scope bootstrap tests.

use super::test_support::{app_state, build_state, init_repo, test_channel};
use super::{resolve_read_repo_id, resolve_write_repo_id};
use crate::server::session::WsSession;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn read_repo_id_uses_active_local_repo_without_sync_binding() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let repo = init_repo(&dir, &vault, "default", Some("urn:default"))?;
    let test_repo = init_repo(&dir, &vault, "test", None)?;
    let test_id = test_repo.get_repo_info()?.expect("test info").uuid;
    let state = app_state(Arc::new(repo), vault);
    let ch = test_channel();
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(test_id));

    assert_eq!(
        resolve_read_repo_id(&state, &ch, &mut session, None),
        Some(test_id)
    );
    assert_eq!(session.active_repo_id, Some(test_id));
    Ok(())
}

#[test]
fn read_repo_id_bootstraps_single_local_repo() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    let ch = test_channel();
    let mut session = WsSession::new();

    assert_eq!(
        resolve_read_repo_id(&state, &ch, &mut session, None),
        Some(default_id)
    );
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[test]
fn write_repo_id_bootstraps_single_local_repo() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    let ch = test_channel();
    let mut session = WsSession::new();

    assert_eq!(
        resolve_write_repo_id(&state, &ch, &mut session, None),
        Some(default_id)
    );
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}
