use super::*;
use crate::server::channel::DualChannel;
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;
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
            path,
            old_content,
            new_content,
        }) => {
            assert_eq!(actual_repo, repo_id);
            assert_eq!(branch.as_ref().map(PeerId::as_str), Some("remote-a"));
            assert_eq!(scope_nonce, Some(7));
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
