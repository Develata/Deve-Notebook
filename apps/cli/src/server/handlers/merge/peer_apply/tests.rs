use super::*;
use crate::server::{AppState, channel::DualChannel, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::protocol::ServerErrorCode;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

#[tokio::test]
async fn merge_conflict_emits_typed_payload_before_diff_fallback() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(8);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let doc_id = DocId::new();
    let repo_id = uuid::Uuid::new_v4();
    let scope = ResolvedRepo {
        repo_id,
        repo_name: "notes".into(),
        branch: Some(PeerId::new("remote-a")),
    };
    let hunk = ConflictHunk {
        start_line: 1,
        length: 2,
        local_lines: vec!["local".into()],
        remote_lines: vec!["remote".into()],
    };

    emit_merge_conflict(
        &ch,
        &scope,
        "docs/a.md".into(),
        MergeConflictPayload {
            doc_id,
            base: "base".into(),
            local: "local".into(),
            remote: "remote".into(),
            conflicts: vec![hunk.clone()],
        },
        Some(7),
    );

    match unicast_rx.recv().await {
        Some(ServerMessage::MergeConflict {
            repo_id: Some(actual_repo),
            branch,
            scope_nonce,
            doc_id: actual_doc,
            path,
            current_content,
            incoming_content,
            result_content,
            actions,
            conflicts,
        }) => {
            assert_eq!(actual_repo, repo_id);
            assert_eq!(branch.as_ref().map(PeerId::as_str), Some("remote-a"));
            assert_eq!(scope_nonce, Some(7));
            assert_eq!(actual_doc, doc_id);
            assert_eq!(path, "docs/a.md");
            assert_eq!(current_content, "local");
            assert_eq!(incoming_content, "remote");
            assert_eq!(result_content, "base");
            assert_eq!(actions.len(), 3);
            assert_eq!(conflicts, vec![hunk]);
        }
        other => panic!("expected typed MergeConflict first, got {other:?}"),
    }

    match unicast_rx.recv().await {
        Some(ServerMessage::DocDiff {
            request_id: None,
            repo_id: Some(actual_repo),
            branch,
            scope_nonce,
            doc_id: actual_doc_id,
            path,
            old_content,
            new_content,
        }) => {
            assert_eq!(actual_repo, repo_id);
            assert_eq!(branch.as_ref().map(PeerId::as_str), Some("remote-a"));
            assert_eq!(scope_nonce, Some(7));
            assert_eq!(actual_doc_id, Some(doc_id));
            assert_eq!(path, "docs/a.md");
            assert_eq!(old_content, "local");
            assert_eq!(new_content, "remote");
        }
        other => panic!("expected DocDiff fallback second, got {other:?}"),
    }

    match unicast_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StorageConflict);
            assert_eq!(scope_nonce, Some(7));
        }
        other => panic!("expected StorageConflict third, got {other:?}"),
    }
}

#[tokio::test]
async fn peer_merge_write_rejects_degraded_local_projection_before_append() -> anyhow::Result<()> {
    let (_dir, state, doc_id, repo_id) = degraded_app_state()?;
    let before = state
        .repo
        .get_local_ops_in_local_repo("default", doc_id)?
        .len();
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(4);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let scope = ResolvedRepo {
        repo_id,
        repo_name: "default".into(),
        branch: None,
    };

    assert!(!write_merged_content(
        &state,
        &ch,
        &scope,
        doc_id,
        "changed",
        Some(13)
    ));

    match unicast_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert_eq!(scope_nonce, Some(13));
        }
        other => panic!("expected degraded projection ProtocolError, got {other:?}"),
    }
    assert_eq!(
        state
            .repo
            .get_local_ops_in_local_repo("default", doc_id)?
            .len(),
        before
    );
    Ok(())
}

fn degraded_app_state() -> anyhow::Result<(TempDir, Arc<AppState>, DocId, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, Some("default"), None)?;
    repo.set_projection_base_for_all_local_repos(&vault);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let (doc_id, _) =
        repo.apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")?;
    repo.append_generated_op_in_local_repo("default", doc_id, PeerId::new("local"), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "base".into(),
            },
            1,
            PeerId::new("local"),
            seq,
            None,
            None,
        )
    })?;
    let repo = Arc::new(repo);
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new(repo.clone()));
    sync_manager.mark_projection_writeback_fault("default");
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager,
            tx: broadcast::channel(4).0,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("local"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key: Arc::new(deve_core::security::IdentityKeyPair::generate()),
        }),
        doc_id,
        repo_id,
    ))
}
