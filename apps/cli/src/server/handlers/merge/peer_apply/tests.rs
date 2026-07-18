use super::*;
use crate::server::{AppState, channel::DualChannel, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{FactActor, Op, PeerId};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

#[tokio::test]
async fn merge_conflict_emits_typed_payload_before_diff_fallback() {
    let (_dir, state, _seeded_doc_id, seeded_repo_id) = degraded_app_state().unwrap();
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(8);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let doc_id = DocId::new();
    let repo_id = seeded_repo_id;
    let scope = ResolvedRepo {
        repo_id,
        repo_name: "notes".into(),
        session_name: "notes".into(),
        branch: Some(PeerId::new("remote-a")),
    };
    let hunk = ConflictHunk {
        start_line: 1,
        length: 2,
        local_lines: vec!["local".into()],
        remote_lines: vec!["remote".into()],
    };

    let mut session = crate::server::session::WsSession::new();
    emit_merge_conflict(
        &state,
        &ch,
        &mut session,
        &scope,
        "docs/a.md".into(),
        MergeConflictPayload {
            doc_id,
            local: "local".into(),
            remote: "remote".into(),
            conflicts: vec![hunk.clone()],
        },
        Some(7),
    );

    let mut saw_projection = false;
    let mut saw_error = false;
    for _ in 0..2 {
        match tokio::time::timeout(std::time::Duration::from_secs(2), unicast_rx.recv())
            .await
            .expect("merge response timeout")
        {
            Some(ServerMessage::MergeConflict {
                repo_id: Some(actual_repo),
                branch,
                scope_nonce,
                doc_id: actual_doc,
                path,
                projection,
                result_content,
                actions,
                conflicts,
            }) => {
                assert_eq!(actual_repo, repo_id);
                assert_eq!(branch.as_ref().map(PeerId::as_str), Some("remote-a"));
                assert_eq!(scope_nonce, Some(7));
                assert_eq!(actual_doc, doc_id);
                assert_eq!(path, "docs/a.md");
                assert_eq!(projection.base_content, "local");
                assert_eq!(projection.target_content, "remote");
                assert_eq!(result_content, "local\nremote");
                assert_eq!(actions.len(), 3);
                assert_eq!(conflicts, vec![hunk.clone()]);
                saw_projection = true;
            }
            Some(ServerMessage::ProtocolError {
                error, scope_nonce, ..
            }) => {
                assert_eq!(error.code, ServerErrorCode::StorageConflict);
                assert_eq!(scope_nonce, Some(7));
                saw_error = true;
            }
            other => panic!("expected typed merge response, got {other:?}"),
        }
    }
    assert!(saw_projection);
    assert!(saw_error);
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
        session_name: "default".into(),
        branch: None,
    };
    let preflight =
        crate::server::session::test_merge_preflight(repo_id, doc_id, "base", "changed");
    let admission = state.repo_mutation_gate().admit_mounted_repo(repo_id)?;

    assert_eq!(
        write_merged_content(
            &state,
            &ch,
            MergeWriteRequest {
                scope: &scope,
                admission,
                preflight: &preflight,
                content: "changed",
                resolution: MergeResolution::AcceptIncoming,
                scope_nonce: Some(13),
            }
        )
        .await,
        MergeWriteOutcome::CommitFailed
    );

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
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, Some("default"), None)?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let (doc_id, _) =
        repo.apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")?;
    repo.local_fact_writer(FactActor::new("test")?)
        .append_content_in_local_repo(
            "default",
            doc_id,
            Op::Insert {
                pos: 0,
                content: "base".into(),
            },
            1,
        )?;
    let repo = Arc::new(repo);
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?);
    sync_manager.mark_projection_writeback_fault(repo.local_repo_name())?;
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
