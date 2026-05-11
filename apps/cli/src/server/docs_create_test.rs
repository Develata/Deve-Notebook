//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 06_repository#repo-scope-runtime

use super::docs_test_support::{
    browser_session, channel, docs_harness, local_session, recv_protocol_error,
    stale_browser_scope_session,
};
use super::handlers::docs::handle_create_doc;
use deve_core::protocol::ServerErrorCode;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_rejects_existing_workspace_file_without_backfill() -> anyhow::Result<()> {
    let h = docs_harness()?;
    let path = h.vault_path("external.md");
    std::fs::create_dir_all(path.parent().expect("parent"))?;
    std::fs::write(&path, "external only")?;
    let (ch, mut rx) = channel(&h.state);
    let mut session = local_session(&h.state, h.repo_id);

    handle_create_doc(&h.state, &ch, &mut session, "external.md".into()).await;

    let (error, _) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::StorageConflict);
    assert!(h.state.repo.get_docid("external.md")?.is_none());
    assert_eq!(std::fs::read_to_string(path)?, "external only");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_rejects_stale_browser_scope_with_scoped_error() -> anyhow::Result<()> {
    let h = docs_harness()?;
    let (ch, mut rx) = channel(&h.state);
    let mut session = stale_browser_scope_session(&h.state, h.repo_id, 17);

    handle_create_doc(&h.state, &ch, &mut session, "scoped.md".into()).await;

    let (error, scope_nonce) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(scope_nonce, Some(17));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_rejects_degraded_local_projection_before_mutation() -> anyhow::Result<()> {
    let h = docs_harness()?;
    h.state
        .sync_manager
        .mark_projection_writeback_fault("default");
    let (ch, mut rx) = channel(&h.state);
    let mut session = browser_session(&h.state, h.repo_id, 29);

    handle_create_doc(&h.state, &ch, &mut session, "blocked.md".into()).await;

    let (error, scope_nonce) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
    assert_eq!(scope_nonce, Some(29));
    assert!(h.state.repo.get_docid("blocked.md")?.is_none());
    assert!(!h.vault_path("blocked.md").exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_rejects_invalid_browser_path_with_scoped_error() -> anyhow::Result<()> {
    let h = docs_harness()?;
    let (ch, mut rx) = channel(&h.state);
    let mut session = browser_session(&h.state, h.repo_id, 23);

    handle_create_doc(&h.state, &ch, &mut session, "../escape.md".into()).await;

    let (error, scope_nonce) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::RequestFailed);
    assert_eq!(scope_nonce, Some(23));
    Ok(())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_doc_fails_closed_when_target_path_is_unstatable() -> anyhow::Result<()> {
    let h = docs_harness()?;
    let blocked = h.vault_path("blocked");
    std::fs::create_dir_all(&blocked)?;
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000))?;
    let (ch, mut rx) = channel(&h.state);
    let mut session = local_session(&h.state, h.repo_id);

    handle_create_doc(&h.state, &ch, &mut session, "blocked/new.md".into()).await;

    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o755))?;
    let (error, _) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::RequestFailed);
    assert!(
        error
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("Failed to check create target"),
        "unexpected detail: {:?}",
        error.detail
    );
    assert!(h.state.repo.get_docid("blocked/new.md")?.is_none());
    Ok(())
}
