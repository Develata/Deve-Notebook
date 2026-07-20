//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime

use super::{session::WsSession, AppState};
use deve_core::models::{DocId, FactActor, Op, RepoId};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use std::sync::Arc;
use tokio::sync::mpsc;

pub(super) const DOC_PATH: &str = "notes/a.md";

pub(super) fn seed_doc(
    state: &Arc<AppState>,
    repo_name: &str,
    content: &str,
) -> anyhow::Result<DocId> {
    let (doc_id, _ops) = state
        .repo
        .apply_file_structure_in_local_repo(repo_name, DOC_PATH, None, "test")?;
    state
        .repo
        .local_fact_writer(FactActor::new("test")?)
        .append_content_in_local_repo(
            repo_name,
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
        )?;
    Ok(doc_id)
}

pub(super) fn delete_doc(state: &Arc<AppState>, doc_id: DocId) -> anyhow::Result<RepoId> {
    let repo_id = state.repo.get_repo_info()?.expect("default info").uuid;
    state.repo.apply_file_delete_structure_in_local_repo(
        state.repo.local_repo_name(),
        DOC_PATH,
        Some(doc_id),
        "test",
    )?;
    Ok(repo_id)
}

pub(super) fn browser_repo_session(
    repo_name: &str,
    repo_id: RepoId,
    scope_nonce: u64,
) -> WsSession {
    let mut session = repo_session(repo_name, repo_id);
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

pub(super) fn repo_session(repo_name: &str, repo_id: RepoId) -> WsSession {
    let mut session = WsSession::new();
    session.switch_repo(repo_name.into(), Some(repo_id));
    session
}

pub(super) async fn assert_protocol_error(
    rx: &mut mpsc::Receiver<ServerMessage>,
    expected_code: Option<ServerErrorCode>,
    expected_scope_nonce: Option<u64>,
    no_extra_label: &str,
) {
    match rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            if let Some(code) = expected_code {
                assert_eq!(error.code, code);
            }
            assert_eq!(scope_nonce, expected_scope_nonce);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(rx.try_recv().is_err(), "{no_extra_label}");
}
