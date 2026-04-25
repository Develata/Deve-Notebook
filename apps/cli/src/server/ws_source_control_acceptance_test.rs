//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 07_diff_logic#source-control-runtime

use super::ws_protocol_acceptance_support::{
    recv_server_message, send_client_message, switch_to_notes_repo,
};
use super::ws_source_control_acceptance_support::{
    SourceControlWsHarness, TestWs, send_scoped,
};
use deve_core::protocol::{ClientMessage, ScPathTarget, ServerErrorCode, ServerMessage};
use deve_core::source_control::ChangeEntry;

const SCOPE: u64 = 1;
const PATH: &str = "sc/ws-added.md";
const CONTENT: &str = "hello from ws\n";
const COMMIT_MESSAGE: &str = "commit through ws";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_source_control_stage_commit_history_roundtrip() -> anyhow::Result<()> {
    let harness = SourceControlWsHarness::spawn().await?;
    harness.seed_pending_added(PATH, CONTENT)?;
    let mut ws = harness.connect().await?;

    switch_to_notes_repo(&mut ws, harness.repo_id, SCOPE).await?;
    request_changes(&mut ws, "before").await?;
    assert_changes(
        recv_server_message(&mut ws).await?,
        harness.repo_id,
        "before",
        &[],
        &[PATH],
    );

    send_scoped(
        &mut ws,
        |scope_nonce| ClientMessage::StageFile {
            target: ScPathTarget::from_path(PATH),
            scope_nonce,
        },
        SCOPE,
    )
    .await?;
    assert_stage_ack(recv_server_message(&mut ws).await?, harness.repo_id);

    request_changes(&mut ws, "staged").await?;
    assert_changes(
        recv_server_message(&mut ws).await?,
        harness.repo_id,
        "staged",
        &[PATH],
        &[],
    );

    send_scoped(
        &mut ws,
        |scope_nonce| ClientMessage::Commit {
            message: COMMIT_MESSAGE.into(),
            scope_nonce,
        },
        SCOPE,
    )
    .await?;
    let commit_id = assert_commit_ack(recv_server_message(&mut ws).await?, harness.repo_id);

    request_history(&mut ws).await?;
    assert_history(
        recv_server_message(&mut ws).await?,
        harness.repo_id,
        &commit_id,
    );
    request_changes(&mut ws, "after").await?;
    assert_changes(
        recv_server_message(&mut ws).await?,
        harness.repo_id,
        "after",
        &[],
        &[],
    );

    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_source_control_rejects_missing_scope_nonce() -> anyhow::Result<()> {
    let harness = SourceControlWsHarness::spawn().await?;
    let mut ws = harness.connect().await?;
    switch_to_notes_repo(&mut ws, harness.repo_id, SCOPE).await?;

    send_client_message(
        &mut ws,
        ClientMessage::GetChanges {
            request_id: "missing-scope".into(),
            scope_nonce: None,
        },
    )
    .await?;
    assert_scope_error(
        recv_server_message(&mut ws).await?,
        "source control scope nonce missing",
    );

    harness.shutdown().await;
    Ok(())
}

async fn request_changes(ws: &mut TestWs, request_id: &str) -> anyhow::Result<()> {
    send_scoped(
        ws,
        |scope_nonce| ClientMessage::GetChanges {
            request_id: request_id.into(),
            scope_nonce,
        },
        SCOPE,
    )
    .await
}

async fn request_history(ws: &mut TestWs) -> anyhow::Result<()> {
    send_client_message(
        ws,
        ClientMessage::GetCommitHistory {
            request_id: "history".into(),
            limit: 10,
            scope_nonce: Some(SCOPE),
        },
    )
    .await
}

fn assert_changes(
    message: ServerMessage,
    repo_id: uuid::Uuid,
    request_id: &str,
    staged_paths: &[&str],
    unstaged_paths: &[&str],
) {
    match message {
        ServerMessage::ChangesList {
            request_id: Some(actual_request),
            repo_id: Some(actual_repo),
            branch,
            scope_nonce,
            staged,
            unstaged,
        } => {
            assert_eq!((actual_repo, branch, scope_nonce), (repo_id, None, Some(SCOPE)));
            assert_eq!(actual_request, request_id);
            assert_eq!(paths(staged), staged_paths);
            assert_eq!(paths(unstaged), unstaged_paths);
        }
        other => panic!("expected ChangesList, got {other:?}"),
    }
}

fn assert_stage_ack(message: ServerMessage, repo_id: uuid::Uuid) {
    match message {
        ServerMessage::StageAck {
            repo_id: Some(actual),
            branch,
            scope_nonce,
            path,
        } => {
            assert_eq!((actual, branch, scope_nonce), (repo_id, None, Some(SCOPE)));
            assert_eq!(path, PATH);
        }
        other => panic!("expected StageAck, got {other:?}"),
    }
}

fn assert_commit_ack(message: ServerMessage, repo_id: uuid::Uuid) -> String {
    match message {
        ServerMessage::CommitAck {
            repo_id: Some(actual),
            branch,
            scope_nonce,
            commit_id,
            timestamp,
        } => {
            assert_eq!((actual, branch, scope_nonce), (repo_id, None, Some(SCOPE)));
            assert!(!commit_id.is_empty());
            assert!(timestamp > 0);
            commit_id
        }
        other => panic!("expected CommitAck, got {other:?}"),
    }
}

fn assert_history(message: ServerMessage, repo_id: uuid::Uuid, commit_id: &str) {
    match message {
        ServerMessage::CommitHistory {
            request_id: Some(request_id),
            repo_id: Some(actual),
            branch,
            scope_nonce,
            commits,
        } => {
            assert_eq!((actual, branch, scope_nonce), (repo_id, None, Some(SCOPE)));
            assert_eq!(request_id, "history");
            let commit = commits
                .iter()
                .find(|commit| commit.id == commit_id)
                .expect("history must include committed id");
            assert_eq!(commit.message, COMMIT_MESSAGE);
            assert!(commit.ledger_seq > 0);
        }
        other => panic!("expected CommitHistory, got {other:?}"),
    }
}

fn assert_scope_error(message: ServerMessage, detail: &str) {
    match message {
        ServerMessage::ProtocolError {
            error, scope_nonce, ..
        } => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(error.detail.as_deref(), Some(detail));
            assert_eq!(scope_nonce, Some(SCOPE));
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
}

fn paths(entries: Vec<ChangeEntry>) -> Vec<String> {
    entries.into_iter().map(|entry| entry.path).collect()
}
