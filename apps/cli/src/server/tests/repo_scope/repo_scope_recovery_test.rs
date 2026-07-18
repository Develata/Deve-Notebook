use super::repo_scope::resolve_session_repo_and_sync;
use super::repo_scope_recovery_support::{build_state, seed_remote_shadow};
use super::session::WsSession;
use deve_core::models::PeerId;

#[test]
fn resolve_session_repo_recovers_remote_repo_name_from_uuid() -> anyhow::Result<()> {
    let (_dir, state, _default_id, remote_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "shadow-notes")?;
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.active_repo_id = Some(remote_repo_id);

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;

    assert_eq!(resolved.branch, Some(peer_id));
    assert_eq!(resolved.repo_id, remote_repo_id);
    assert_eq!(resolved.repo_name, remote_repo_id.to_string());
    assert_eq!(resolved.session_name, "shadow-notes");
    assert_eq!(session.active_repo.as_deref(), Some("shadow-notes"));
    Ok(())
}

#[test]
fn resolve_session_repo_rejects_local_uuid_string_selector_without_bound_id() -> anyhow::Result<()>
{
    let (_dir, state, _default_id, test_id) = build_state()?;
    let mut session = WsSession::new();
    session.switch_repo(test_id.to_string(), None);

    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("local uuid-string selector must fail closed");
    assert!(
        err.to_string()
            .contains("Local repository selector not resolved")
    );
    assert_eq!(session.active_repo, None);
    assert_eq!(session.active_repo_id, None);
    Ok(())
}

#[test]
fn resolve_session_repo_accepts_bound_canonical_local_uuid() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let execution_name = test_id.to_string();
    let mut session = WsSession::new();
    session.switch_repo(execution_name.clone(), Some(test_id));

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;

    assert_eq!(resolved.repo_id, test_id);
    assert_eq!(resolved.repo_name, execution_name);
    assert_eq!(resolved.session_name, "test");
    assert_eq!(session.active_repo.as_deref(), Some("test"));
    assert_eq!(session.active_repo_id, Some(test_id));
    Ok(())
}

#[test]
fn resolve_session_repo_fails_closed_when_remote_name_is_stale_even_with_bound_uuid()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, remote_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "shadow-notes")?;
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("stale-name".into(), Some(remote_repo_id));

    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("remote stale selector with bound uuid must fail closed");
    assert!(
        err.to_string()
            .contains("Remote repository selector not resolved")
    );
    Ok(())
}

#[test]
fn resolve_session_repo_rejects_remote_selector_without_uuid_when_name_is_unrecoverable()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, remote_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "shadow-notes")?;
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("stale-name".into(), None);

    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("remote stale selector without uuid must fail");
    assert!(
        err.to_string()
            .contains("Remote repository selector not resolved")
    );
    Ok(())
}

#[test]
fn resolve_session_repo_rejects_remote_uuid_string_without_bound_id() -> anyhow::Result<()> {
    let (_dir, state, _default_id, remote_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "shadow-notes")?;
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo(remote_repo_id.to_string(), None);

    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("remote uuid-shaped repo_name without repo_id must fail closed");
    assert!(
        err.to_string()
            .contains("Remote repository selector not resolved")
    );
    Ok(())
}
