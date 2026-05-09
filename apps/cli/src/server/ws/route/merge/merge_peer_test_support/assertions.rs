use deve_core::models::{DocId, PeerId};
use deve_core::protocol::{MergeConflictAction, ServerMessage};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, timeout};

pub(crate) struct MergeConflictExpectation<'a> {
    pub(crate) repo_id: uuid::Uuid,
    pub(crate) branch: Option<PeerId>,
    pub(crate) scope_nonce: Option<u64>,
    pub(crate) doc_id: DocId,
    pub(crate) path: &'a str,
    pub(crate) current_content: &'a str,
    pub(crate) incoming_content: &'a str,
    pub(crate) result_content: &'a str,
    pub(crate) start_line: usize,
    pub(crate) length: usize,
    pub(crate) local_lines: &'a [&'a str],
    pub(crate) remote_lines: &'a [&'a str],
}

pub(crate) async fn expect_merge_complete(
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

pub(crate) async fn expect_merge_conflict(
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
