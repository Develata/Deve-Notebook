//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Merge scope stale binding tests.

use super::resolve_read_repo_id;
use super::test_support::{build_state, test_channel};
use crate::server::session::WsSession;
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::PeerId;

#[test]
fn read_repo_id_bootstraps_after_clearing_stale_local_binding() -> anyhow::Result<()> {
    let (_dir, state, default_id) = build_state()?;
    let ch = test_channel();
    let mut session = WsSession::new();
    session.set_active_db(DatabaseHandle::local(uuid::Uuid::new_v4(), "ghost".into()));
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(uuid::Uuid::new_v4());
    session.set_sync_scope_nonce(11);

    assert_eq!(
        resolve_read_repo_id(&state, &ch, &mut session, None),
        Some(default_id)
    );
    assert_eq!(
        session.active_repo.as_deref(),
        Some(default_id.to_string().as_str())
    );
    assert!(session.get_active_db().is_none());
    Ok(())
}
