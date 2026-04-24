//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 06_repository#repo-scope-runtime

use super::docs_seed_test_support::seed_file;
use super::docs_test_support::{channel, docs_harness, local_session, recv_protocol_error};
use super::handlers::docs::{
    handle_copy_doc, handle_create_doc, handle_move_doc, handle_rename_doc,
};
use deve_core::protocol::ServerErrorCode;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rename_recovers_from_missing_source_projection() -> anyhow::Result<()> {
    let h = docs_harness()?;
    seed_file(&h, "notes/a.md", "hello")?;
    let (ch, _rx) = channel(&h.state);
    let mut session = local_session(&h.state, h.repo_id);
    handle_rename_doc(
        &h.state,
        &ch,
        &mut session,
        "notes/a.md".into(),
        "notes/b.md".into(),
    )
    .await;
    assert_eq!(h.state.repo.get_docid("notes/a.md")?, None);
    assert_eq!(std::fs::read_to_string(h.vault_path("notes/b.md"))?, "hello");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn copy_recovers_from_missing_source_projection() -> anyhow::Result<()> {
    let h = docs_harness()?;
    seed_file(&h, "notes/a.md", "hello")?;
    let (ch, _rx) = channel(&h.state);
    let mut session = local_session(&h.state, h.repo_id);
    handle_copy_doc(
        &h.state,
        &ch,
        &mut session,
        "notes/a.md".into(),
        "notes/b.md".into(),
    )
    .await;
    assert_eq!(std::fs::read_to_string(h.vault_path("notes/b.md"))?, "hello");
    assert!(h.state.repo.get_docid("notes/b.md")?.is_some());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_rejects_existing_tracked_path_without_projection() -> anyhow::Result<()> {
    let h = docs_harness()?;
    seed_file(&h, "notes/a.md", "ledger only")?;
    let (ch, _rx) = channel(&h.state);
    let mut session = local_session(&h.state, h.repo_id);
    let original = h.state.repo.get_docid("notes/a.md")?;
    handle_create_doc(&h.state, &ch, &mut session, "notes/a.md".into()).await;
    assert_eq!(h.state.repo.get_docid("notes/a.md")?, original);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_rejects_same_source_and_destination() -> anyhow::Result<()> {
    let h = docs_harness()?;
    seed_file(&h, "notes/a.md", "hello")?;
    let (ch, mut rx) = channel(&h.state);
    let mut session = local_session(&h.state, h.repo_id);

    handle_move_doc(
        &h.state,
        &ch,
        &mut session,
        "notes/a.md".into(),
        "notes/a.md".into(),
    )
    .await;

    let (error, _) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::RequestFailed);
    assert_eq!(
        error.detail.as_deref(),
        Some("Destination must differ from source")
    );
    assert!(h.state.repo.get_docid("notes/a.md")?.is_some());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_trims_source_and_destination_paths() -> anyhow::Result<()> {
    let h = docs_harness()?;
    seed_file(&h, "notes/a.md", "hello")?;
    let (ch, _rx) = channel(&h.state);
    let mut session = local_session(&h.state, h.repo_id);

    handle_move_doc(
        &h.state,
        &ch,
        &mut session,
        "  notes/a.md  ".into(),
        "  notes/b  ".into(),
    )
    .await;

    assert_eq!(h.state.repo.get_docid("notes/a.md")?, None);
    assert!(h.state.repo.get_docid("notes/b.md")?.is_some());
    assert!(h.vault_path("notes/b.md").exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_rejects_traversal_source_before_resolving_target() -> anyhow::Result<()> {
    let h = docs_harness()?;
    let (ch, mut rx) = channel(&h.state);
    let mut session = local_session(&h.state, h.repo_id);

    handle_move_doc(
        &h.state,
        &ch,
        &mut session,
        "../secret.md".into(),
        "notes/b.md".into(),
    )
    .await;

    let (error, _) = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::RequestFailed);
    assert_eq!(error.detail.as_deref(), Some("Invalid path: ../secret.md"));
    Ok(())
}
