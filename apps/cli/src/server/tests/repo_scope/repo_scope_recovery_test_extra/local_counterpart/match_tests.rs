//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::support::{build_state, remote_scope, seed_remote_shadow};
use crate::server::repo_scope::resolve_local_counterpart_repo;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;

#[test]
fn resolve_local_counterpart_repo_prefers_repo_uuid_for_remote_scope() -> anyhow::Result<()> {
    let (_dir, state, _default_id, remote_repo_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    seed_remote_shadow(&state, &peer_id, remote_repo_id, "shadow-notes")?;

    let local = resolve_local_counterpart_repo(
        &state,
        &remote_scope(remote_repo_id, "shadow-notes", peer_id),
    )?
    .expect("local counterpart");

    assert!(local.branch.is_none());
    assert_eq!(local.repo_name, remote_repo_id.to_string());
    assert_eq!(local.session_name, "test");
    assert_eq!(local.repo_id, remote_repo_id);
    Ok(())
}

#[test]
fn resolve_local_counterpart_repo_requires_uuid_or_url_match() -> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let remote_repo_id = uuid::Uuid::new_v4();
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: remote_repo_id,
            name: "test".into(),
            url: Some("urn:remote-only".into()),
        },
    )?;

    let local = resolve_local_counterpart_repo(
        &state,
        &remote_scope(remote_repo_id, "test", peer_id),
    )?;
    assert!(local.is_none());
    Ok(())
}

#[test]
fn resolve_local_counterpart_repo_fails_closed_when_resolved_remote_scope_has_no_url()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let remote_repo_id = uuid::Uuid::new_v4();
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: remote_repo_id,
            name: "shadow-no-url".into(),
            url: None,
        },
    )?;

    let err = resolve_local_counterpart_repo(
        &state,
        &remote_scope(remote_repo_id, "shadow-no-url", peer_id),
    )
    .expect_err("remote scope without URL must fail closed");
    assert!(err.to_string().contains("repository URL missing"));
    Ok(())
}

#[test]
fn resolve_local_counterpart_repo_uses_unique_local_url_after_catalog_repair() -> anyhow::Result<()>
{
    let (dir, state, _default_id, _test_id) = build_state()?;
    let _dup = RepoManager::init(dir.path(), 10, Some("mirror"), Some("urn:test"))?;
    let peer_id = PeerId::new("peer-a");
    let remote_repo_id = uuid::Uuid::new_v4();
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: remote_repo_id,
            name: "shadow-test".into(),
            url: Some("urn:test".into()),
        },
    )?;
    let expected_local = state
        .repo
        .find_local_repo_name_by_url("urn:test")?
        .expect("repair must leave a unique local URL owner");
    let expected_info = state
        .repo
        .get_repo_info_for(None, Some(&expected_local))?
        .expect("local repo info");

    let local = resolve_local_counterpart_repo(
        &state,
        &remote_scope(remote_repo_id, "shadow-test", peer_id),
    )?
    .expect("local counterpart should recover via unique URL");

    assert!(local.branch.is_none());
    assert_eq!(local.repo_name, expected_local);
    assert_eq!(local.repo_id, expected_info.uuid);
    Ok(())
}
