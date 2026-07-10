//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::super::handlers::sync::handle_sync_hello;
use super::super::sync_hello_test_support::{
    build_state, collect_unicast_messages, empty_session, signed_hello_for_repo,
    signed_hello_for_scope, unicast_channel,
};
use deve_core::ledger::listing::RepoListing;
use deve_core::security::IdentityKeyPair;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_creates_repo_scoped_shadow_without_borrowing_local_metadata()
-> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&remote, repo_id);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let _ = rx.recv().await;

    assert!(state.repo.list_repos(Some(&remote.peer_id()))?.is_empty());
    assert!(
        state
            .repo
            .remotes_dir()
            .join(remote.peer_id().to_filename())
            .join(format!("{repo_id}.redb"))
            .exists()
    );
    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_binds_session_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_scope(&remote, repo_id, 9);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let _ = collect_unicast_messages(&mut rx).await?;

    assert_eq!(session.sync_scope_nonce(), Some(9));
    Ok(())
}
