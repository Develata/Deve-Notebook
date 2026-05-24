use super::status_lines_at;
use deve_core::git_bridge::{
    GitMetadataKind, GitMirrorCommitState, GitMirrorFailureStage, GitMirrorRecord, GitMirrorState,
    GitMirrorStatus, GitMirrorSummary,
};

fn out_of_sync_record(error: &str) -> GitMirrorRecord {
    GitMirrorRecord {
        deve_commit_id: "deve-1".into(),
        repo_id: uuid::Uuid::nil(),
        ledger_seq: 7,
        state: GitMirrorCommitState::OutOfSync,
        git_commit_id: None,
        last_error: Some(error.to_string()),
        failure_stage: Some(GitMirrorFailureStage::classify(error)),
        failure_subject: Some("docs/example.md".to_string()),
        failure_command: None,
        failure_exit_status: None,
        queued_at_ms: 1,
        updated_at_ms: 2,
        attempts: 1,
    }
}

fn out_of_sync_stage(index: usize, stage: GitMirrorFailureStage) -> GitMirrorRecord {
    GitMirrorRecord {
        deve_commit_id: format!("deve-{index}"),
        repo_id: uuid::Uuid::nil(),
        ledger_seq: index as u64,
        state: GitMirrorCommitState::OutOfSync,
        git_commit_id: None,
        last_error: Some("failure".to_string()),
        failure_stage: Some(stage),
        failure_subject: None,
        failure_command: (stage == GitMirrorFailureStage::GitCommand).then(|| "commit".to_string()),
        failure_exit_status: None,
        queued_at_ms: 1,
        updated_at_ms: 2,
        attempts: 1,
    }
}

fn ready_status() -> GitMirrorStatus {
    GitMirrorStatus {
        repo_root: std::path::PathBuf::from("notes/default"),
        notegit_present: true,
        git_metadata_kind: GitMetadataKind::Directory,
        gitignore_protects_notegit: true,
        state: GitMirrorState::Ready,
        reason: None,
    }
}

#[test]
fn status_lines_include_cli_only_repair_action() {
    let lines = status_lines_at(
        "default",
        &ready_status(),
        &GitMirrorSummary {
            queued: 0,
            committed: 0,
            out_of_sync: 1,
        },
        &[out_of_sync_record(
            "Git mirror refuses unsafe projection path: docs/example.md",
        )],
        11,
    );

    assert!(lines.iter().any(|line| {
        line.contains(
            "repair_action[1]: code=resolve_projection_scope retryable_after_fix=yes subject=docs/example.md",
        )
    }));
    assert!(lines.iter().any(|line| {
        line.contains(
            "repair_guidance[1]: manual_only=yes next=fix_projection_or_path_subject retry_command=`deve_cli git export --repo default --retry-out-of-sync`",
        )
    }));
    assert!(lines.iter().any(|line| {
        line.contains(
            "repair_hint: fix the reported repair_action subject, then run `deve_cli git export --repo default --retry-out-of-sync`",
        )
    }));
}

#[test]
fn status_lines_include_guidance_for_all_repair_actions() {
    let stages = [
        GitMirrorFailureStage::MirrorNotReady,
        GitMirrorFailureStage::DeveSourceControl,
        GitMirrorFailureStage::NotegitProtection,
        GitMirrorFailureStage::ProjectionScope,
        GitMirrorFailureStage::GitHistoryMapping,
        GitMirrorFailureStage::GitWorktree,
        GitMirrorFailureStage::GitCommand,
        GitMirrorFailureStage::MirrorExecutor,
    ];
    let records = stages
        .into_iter()
        .enumerate()
        .map(|(index, stage)| out_of_sync_stage(index + 1, stage))
        .collect::<Vec<_>>();
    let lines = status_lines_at(
        "default",
        &ready_status(),
        &GitMirrorSummary {
            queued: 0,
            committed: 0,
            out_of_sync: records.len(),
        },
        &records,
        11,
    );

    for expected in [
        "next=prepare_git_mirror_and_notegit_gitignore",
        "next=stage_commit_or_discard_deve_source_control_changes",
        "next=remove_notegit_from_git_tracking_and_restore_gitignore",
        "next=fix_projection_or_path_subject",
        "next=repair_git_history_mapping_or_rebootstrap_empty_mirror",
        "next=clean_git_worktree_or_import_changes",
        "next=inspect_git_command_failure",
        "next=inspect_mirror_executor_error",
    ] {
        assert!(
            lines
                .iter()
                .any(|line| line.contains("repair_guidance") && line.contains(expected)),
            "missing {expected}: {lines:?}"
        );
    }

    assert!(lines.iter().any(|line| {
        line.contains("repair_action[1]: code=prepare_mirror")
            && line.contains("subject=mirror_readiness")
    }));
    assert!(lines.iter().any(|line| {
        line.contains("repair_action[7]: code=inspect_git_command")
            && line.contains("subject=commit")
    }));
    assert!(lines.iter().any(|line| {
        line.contains("repair_guidance[8]: manual_only=yes") && line.contains("retry_command=-")
    }));
}
