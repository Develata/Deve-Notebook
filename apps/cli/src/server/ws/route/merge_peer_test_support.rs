//! Shared peer merge route test support.

use super::route_merge;
use crate::server::session::WsSession;
use crate::server::{AppState, channel::DualChannel, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId, RepoType};
use deve_core::protocol::{ClientMessage, MergeConflictAction, ServerMessage};
use deve_core::sync::{SyncManager, repo_scoped::RepoScopedSyncEngine};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, timeout};

pub(super) struct MergeConflictExpectation<'a> {
    pub(super) repo_id: uuid::Uuid,
    pub(super) branch: Option<PeerId>,
    pub(super) scope_nonce: Option<u64>,
    pub(super) doc_id: DocId,
    pub(super) path: &'a str,
    pub(super) current_content: &'a str,
    pub(super) incoming_content: &'a str,
    pub(super) result_content: &'a str,
    pub(super) start_line: usize,
    pub(super) length: usize,
    pub(super) local_lines: &'a [&'a str],
    pub(super) remote_lines: &'a [&'a str],
}

pub(super) fn reopen_state(root: &Path) -> anyhow::Result<Arc<AppState>> {
    let vault = root.join("vault");
    let mut repo = RepoManager::init(root, 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&root.join("host"))?;
    Ok(Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(SyncManager::new(repo.clone(), vault)),
        tx,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            identity_key.peer_id(),
            repo,
            SyncMode::Auto,
        )),
        tree_manager: Arc::new(RepoTreeRegistry::new()),
        #[cfg(feature = "search")]
        search_available: false,
        identity_key,
    }))
}

pub(super) async fn request_merge_peer(
    state: &std::sync::Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: &PeerId,
    doc_id: DocId,
    scope_nonce: u64,
) {
    route_merge(
        state,
        ch,
        session,
        ClientMessage::MergePeer {
            peer_id: peer_id.to_string(),
            doc_id,
            scope_nonce: Some(scope_nonce),
        },
    )
    .await;
}

pub(super) fn ensure_remote_repo(
    state: &std::sync::Arc<AppState>,
    repo_id: uuid::Uuid,
) -> anyhow::Result<PeerId> {
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "notes".into(),
            url: Some("urn:test:notes".into()),
        },
    )?;
    Ok(peer_id)
}

pub(super) fn seed_local_doc(
    state: &std::sync::Arc<AppState>,
    path: &str,
) -> anyhow::Result<DocId> {
    let (doc_id, _ops) = state
        .repo
        .apply_file_structure_in_local_repo("notes", path, None, "test")?;
    Ok(doc_id)
}

pub(super) fn seed_remote_insert(
    state: &std::sync::Arc<AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    doc_id: DocId,
    content: &str,
) -> anyhow::Result<()> {
    state.repo.append_remote_op(
        peer_id,
        &repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
            peer_id.clone(),
            1,
            None,
            None,
        ),
    )?;
    Ok(())
}

pub(super) fn seed_shared_base(
    state: &std::sync::Arc<AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    doc_id: DocId,
    content: &str,
) -> anyhow::Result<()> {
    let base_entry = LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos: 0,
            content: content.into(),
        },
        1,
        PeerId::new("shared-base"),
        1,
        None,
        None,
    );
    state
        .repo
        .append_local_op_in_local_repo("notes", &base_entry)?;
    state
        .repo
        .append_remote_op(peer_id, &repo_id, &base_entry)?;
    Ok(())
}

pub(super) fn seed_local_replace(
    state: &std::sync::Arc<AppState>,
    doc_id: DocId,
    before: &str,
    after: &str,
) -> anyhow::Result<()> {
    let peer_id = PeerId::new("local-test");
    state.repo.append_local_op_in_local_repo(
        "notes",
        &LedgerEntry::new_content(
            doc_id,
            Op::Delete {
                pos: 0,
                len: utf16_len(before),
            },
            2,
            peer_id.clone(),
            1,
            None,
            None,
        ),
    )?;
    state.repo.append_local_op_in_local_repo(
        "notes",
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: after.into(),
            },
            3,
            peer_id,
            2,
            None,
            None,
        ),
    )?;
    Ok(())
}

pub(super) fn seed_remote_replace(
    state: &std::sync::Arc<AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    doc_id: DocId,
    before: &str,
    after: &str,
) -> anyhow::Result<()> {
    state.repo.append_remote_op(
        peer_id,
        &repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Delete {
                pos: 0,
                len: utf16_len(before),
            },
            2,
            peer_id.clone(),
            1,
            None,
            None,
        ),
    )?;
    state.repo.append_remote_op(
        peer_id,
        &repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: after.into(),
            },
            3,
            peer_id.clone(),
            2,
            None,
            None,
        ),
    )?;
    Ok(())
}

pub(super) fn browser_remote_session(
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(scope_nonce));
    session
}

pub(super) fn local_doc_content(
    state: &std::sync::Arc<AppState>,
    doc_id: DocId,
) -> anyhow::Result<(usize, String)> {
    let entries = state
        .repo
        .get_local_ops_in_local_repo("notes", doc_id)?
        .into_iter()
        .map(|(_, entry)| entry)
        .filter(|entry| entry.content_op().is_some())
        .collect::<Vec<_>>();
    Ok((
        entries.len(),
        deve_core::state::reconstruct_content(&entries),
    ))
}

pub(super) fn local_doc_entry_count(
    state: &std::sync::Arc<AppState>,
    doc_id: DocId,
) -> anyhow::Result<usize> {
    Ok(state
        .repo
        .get_local_ops_in_local_repo("notes", doc_id)?
        .len())
}

pub(super) fn doc_content(
    state: &std::sync::Arc<AppState>,
    repo_type: RepoType,
    doc_id: DocId,
) -> anyhow::Result<(usize, String)> {
    let entries = state
        .repo
        .get_ops(&repo_type, doc_id)?
        .into_iter()
        .map(|(_, entry)| entry)
        .filter(|entry| entry.content_op().is_some())
        .collect::<Vec<_>>();
    Ok((
        entries.len(),
        deve_core::state::reconstruct_content(&entries),
    ))
}

pub(super) fn doc_entry_count(
    state: &std::sync::Arc<AppState>,
    repo_type: RepoType,
    doc_id: DocId,
) -> anyhow::Result<usize> {
    Ok(state.repo.get_ops(&repo_type, doc_id)?.len())
}

pub(super) async fn expect_merge_complete(
    broadcast_rx: &mut broadcast::Receiver<ServerMessage>,
    repo_id: uuid::Uuid,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    merged_count: u32,
) -> anyhow::Result<()> {
    match timeout(Duration::from_secs(2), broadcast_rx.recv()).await?? {
        ServerMessage::MergeComplete {
            repo_id: actual_repo,
            branch: actual_branch,
            scope_nonce: actual_scope_nonce,
            merged_count: actual_count,
        } => {
            assert_eq!(actual_repo, Some(repo_id));
            assert_eq!(actual_branch, branch);
            assert_eq!(actual_scope_nonce, scope_nonce);
            assert_eq!(actual_count, merged_count);
        }
        other => panic!("expected MergeComplete, got {other:?}"),
    }
    Ok(())
}

pub(super) async fn expect_merge_conflict(
    uni_rx: &mut mpsc::Receiver<ServerMessage>,
    expected: MergeConflictExpectation<'_>,
) -> anyhow::Result<()> {
    match timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("merge conflict")
    {
        ServerMessage::MergeConflict {
            repo_id: actual_repo,
            branch: actual_branch,
            scope_nonce: actual_scope_nonce,
            doc_id: actual_doc_id,
            path,
            current_content,
            incoming_content,
            result_content,
            actions,
            conflicts,
        } => {
            assert_eq!(actual_repo, Some(expected.repo_id));
            assert_eq!(actual_branch, expected.branch);
            assert_eq!(actual_scope_nonce, expected.scope_nonce);
            assert_eq!(actual_doc_id, expected.doc_id);
            assert_eq!(path, expected.path);
            assert_eq!(current_content, expected.current_content);
            assert_eq!(incoming_content, expected.incoming_content);
            assert_eq!(result_content, expected.result_content);
            assert_eq!(actions.len(), 3);
            assert!(actions.contains(&MergeConflictAction::AcceptCurrent));
            assert!(actions.contains(&MergeConflictAction::AcceptIncoming));
            assert!(actions.contains(&MergeConflictAction::AcceptBoth));
            assert_eq!(conflicts.len(), 1);
            let conflict = &conflicts[0];
            assert_eq!(conflict.start_line, expected.start_line);
            assert_eq!(conflict.length, expected.length);
            assert_eq!(
                conflict.local_lines,
                expected
                    .local_lines
                    .iter()
                    .map(|line| (*line).to_string())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                conflict.remote_lines,
                expected
                    .remote_lines
                    .iter()
                    .map(|line| (*line).to_string())
                    .collect::<Vec<_>>()
            );
        }
        other => panic!("expected MergeConflict, got {other:?}"),
    }
    Ok(())
}

fn utf16_len(value: &str) -> u32 {
    value.encode_utf16().count() as u32
}
