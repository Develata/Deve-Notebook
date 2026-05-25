//! plan_ref:
//!   - 03_storage#git-ecosystem-coexistence
//!   - 05_diff_logic#source-control-runtime
//!   - 14_commands#cli-commands

use crate::server::{
    channel::DualChannel,
    handlers::source_control::{handle_commit, handle_resolve_conflict},
    session::WsSession,
};
use deve_core::git_bridge::{GitMirrorCommitState, GitMirrorRunOptions, export_mirror, get_record};
use deve_core::protocol::{ScPathTarget, ServerMessage};
use deve_core::source_control::ConflictResolution;
use tokio::sync::mpsc;

use super::source_control_git_import_test_support as support;
use support::{create_mapped_imported_conflict_fixture, git};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolved_import_keep_fs_commits_and_exports_to_git() -> anyhow::Result<()> {
    let fixture = create_mapped_imported_conflict_fixture()?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(fixture.state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(fixture.repo_name.clone(), None);
    session.set_scope_nonce(Some(36));

    handle_resolve_conflict(
        &fixture.state,
        &ch,
        &mut session,
        ScPathTarget {
            path: "note.md".into(),
            doc_id: Some(fixture.doc_id),
        },
        ConflictResolution::KeepFs,
    )
    .await;

    match uni_rx.recv().await.expect("conflict resolved") {
        ServerMessage::ConflictResolved {
            repo_id,
            branch,
            scope_nonce,
            path,
            resolution,
            ..
        } => {
            assert_eq!(repo_id, Some(fixture.repo_id));
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, Some(36));
            assert_eq!(path, "note.md");
            assert_eq!(resolution, "KeepFs");
        }
        other => panic!("expected ConflictResolved, got {other:?}"),
    }
    let mut broadcast_rx = fixture.state.tx.subscribe();
    handle_commit(
        &fixture.state,
        &ch,
        &mut session,
        "accept imported git content".into(),
    )
    .await;

    let committed_id = match broadcast_rx.recv().await.expect("commit ack") {
        ServerMessage::CommitAck {
            repo_id,
            branch,
            scope_nonce,
            commit_id,
            ..
        } => {
            assert_eq!(repo_id, Some(fixture.repo_id));
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, Some(36));
            commit_id
        }
        other => panic!("expected CommitAck, got {other:?}"),
    };
    assert_eq!(
        fixture
            .state
            .repo
            .list_commits_in_local_repo(&fixture.repo_name, 10)?
            .len(),
        fixture.before_commit_count + 1
    );
    assert!(
        fixture
            .state
            .repo
            .list_pending_fs_in_local_repo(&fixture.repo_name)?
            .is_empty()
    );
    assert!(
        fixture
            .state
            .repo
            .list_staged_in_local_repo(&fixture.repo_name)?
            .is_empty()
    );
    fixture
        .state
        .repo
        .run_on_local_repo(&fixture.repo_name, |db| {
            let record = get_record(db, &committed_id)?.expect("queued imported commit");
            assert_eq!(record.state, GitMirrorCommitState::Queued);
            Ok::<_, anyhow::Error>(())
        })?;

    let export_report = fixture
        .state
        .repo
        .run_on_local_repo(&fixture.repo_name, |db| {
            Ok(export_mirror(
                db,
                &fixture.repo_root,
                fixture.repo_id,
                GitMirrorRunOptions::default(),
            )?)
        })?;

    assert_eq!(export_report.attempted, 1, "{export_report:?}");
    assert_eq!(export_report.committed, 1, "{export_report:?}");
    assert_eq!(export_report.out_of_sync, 0, "{export_report:?}");
    fixture
        .state
        .repo
        .run_on_local_repo(&fixture.repo_name, |db| {
            let record = get_record(db, &committed_id)?.expect("committed imported mirror record");
            assert_eq!(record.state, GitMirrorCommitState::Committed);
            assert!(record.git_commit_id.is_some(), "{record:?}");
            Ok::<_, anyhow::Error>(())
        })?;
    assert!(git(&fixture.repo_root, &["status", "--porcelain"]).is_empty());
    assert_eq!(
        git(&fixture.repo_root, &["show", "HEAD:note.md"]),
        "git import\n"
    );
    let head_body = git(&fixture.repo_root, &["log", "-1", "--format=%B"]);
    assert!(head_body.contains(&committed_id), "{head_body}");
    Ok(())
}
