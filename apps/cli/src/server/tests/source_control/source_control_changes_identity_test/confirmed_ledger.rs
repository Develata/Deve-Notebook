//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::support::build_state;
use crate::server::{
    AppState, channel::DualChannel,
    handlers::source_control::{handle_get_changes, handle_get_doc_diff},
    session::WsSession,
};
use deve_core::models::{DocId, FactActor, Op};
use deve_core::protocol::{ScPathTarget, ServerMessage};
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeDomain, ChangeStatus};
use std::sync::Arc;
use tokio::sync::mpsc;

fn seed_confirmed_modified_change(state: &Arc<AppState>) -> anyhow::Result<(String, DocId)> {
    let path = "notes/a.md".to_string();
    let abs = state.repo.local_repo_workspace_path("default", &path)?;
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&abs, "hello")?;

    state.repo.run_on_local_repo("default", |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.clone(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;
    state.repo.stage_pending(&path)?;
    state.repo.apply_external_changes()?;
    state
        .repo
        .commit_source_control_changes("initial")?;

    let doc_id = state
        .repo
        .get_tracked_docid_in_local_repo("default", &path)?
        .expect("tracked doc id after initial commit");
    state
        .repo
        .local_fact_writer(FactActor::new("editor")?)
        .append_content_in_local_repo(
            "default",
            doc_id,
            Op::Insert {
                pos: 5,
                content: " world".into(),
            },
            1000,
        )?;
    Ok((path, doc_id))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn confirmed_ledger_changes_are_sent_as_separate_resource_group() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (path, doc_id) = seed_confirmed_modified_change(&state)?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(31));
    session.switch_repo("default".into(), None);
    handle_get_changes(&state, &ch, &mut session, Some("req-confirmed".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ChangesList {
            scope_nonce,
            staged,
            unstaged,
            confirmed,
            ..
        }) => {
            assert_eq!(scope_nonce, Some(31));
            assert!(staged.is_empty());
            assert!(unstaged.is_empty());
            assert_eq!(confirmed.len(), 1);
            assert_eq!(confirmed[0].domain, ChangeDomain::ConfirmedLedger);
            assert_eq!(confirmed[0].status, ChangeStatus::Modified);
            assert_eq!(confirmed[0].path, path);
            assert_eq!(confirmed[0].doc_id, Some(doc_id));
            assert!(confirmed[0].base_seq.is_some());
            assert!(confirmed[0].target_seq.is_some());
        }
        other => panic!("expected ChangesList, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn confirmed_ledger_doc_diff_uses_commit_anchor_as_left_side() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (path, doc_id) = seed_confirmed_modified_change(&state)?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(41));
    session.switch_repo("default".into(), None);

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "req-confirmed-diff".into(),
        ScPathTarget {
            path: path.clone(),
            doc_id: Some(doc_id),
            domain: Some(ChangeDomain::ConfirmedLedger),
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::DocDiff {
            request_id,
            scope_nonce,
            doc_id: actual_doc_id,
            path: actual_path,
            old_content,
            new_content,
            ..
        }) => {
            assert_eq!(request_id.as_deref(), Some("req-confirmed-diff"));
            assert_eq!(scope_nonce, Some(41));
            assert_eq!(actual_doc_id, Some(doc_id));
            assert_eq!(actual_path, path);
            assert_eq!(old_content, "hello");
            assert_eq!(new_content, "hello world");
        }
        other => panic!("expected DocDiff, got {:?}", other),
    }
    Ok(())
}
