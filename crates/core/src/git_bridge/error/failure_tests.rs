#[test]
fn git_mirror_run_failure_classifies_infra_and_business_failures() {
    match super::GitMirrorRunFailure::from_commit_error(super::GitMirrorCommitError::GitPreflight(
        super::GitPreflightError::CommitDiff {
            message: "lost projected path".into(),
        },
    )) {
        super::GitMirrorRunFailure::OutOfSync(reason) => {
            assert!(
                reason.contains("failed to compute queued Deve commit diff"),
                "{reason}"
            );
        }
        super::GitMirrorRunFailure::Propagate(err) => {
            panic!("CommitDiff should remain mirror out-of-sync, got {err:?}");
        }
    }

    match super::GitMirrorRunFailure::from_commit_error(super::GitMirrorCommitError::GitPreflight(
        super::GitPreflightError::CommitTable {
            action: "open",
            message: "table missing".into(),
        },
    )) {
        super::GitMirrorRunFailure::Propagate(super::GitMirrorRunError::CommitTable {
            action: "open",
            message,
        }) => assert_eq!(message, "table missing"),
        other => panic!(
            "CommitTable should propagate, got {}",
            classify_failure(other)
        ),
    }

    match super::GitMirrorRunFailure::from_commit_error(super::GitMirrorCommitError::GitPreflight(
        super::GitPreflightError::CommitDiffStorage {
            message: "range table missing".into(),
        },
    )) {
        super::GitMirrorRunFailure::Propagate(super::GitMirrorRunError::CommitDiffStorage {
            message,
        }) => assert_eq!(message, "range table missing"),
        other => panic!(
            "CommitDiffStorage should propagate, got {}",
            classify_failure(other)
        ),
    }

    match super::GitMirrorRunFailure::from_commit_error(super::GitMirrorCommitError::GitPreflight(
        super::GitPreflightError::SourceControlInspect {
            kind: "pending",
            message: "table missing".into(),
        },
    )) {
        super::GitMirrorRunFailure::Propagate(super::GitMirrorRunError::SourceControlInspect {
            kind: "pending",
            message,
        }) => assert_eq!(message, "table missing"),
        other => panic!(
            "SourceControlInspect should propagate, got {}",
            classify_failure(other)
        ),
    }

    match super::GitMirrorRunFailure::from_snapshot_bootstrap_error(
        super::GitSnapshotBootstrapError::ProjectionSnapshotInspectStorage {
            message: "range table missing".into(),
        },
    ) {
        super::GitMirrorRunFailure::Propagate(super::GitMirrorRunError::CommitDiffStorage {
            message,
        }) => assert_eq!(message, "range table missing"),
        other => panic!(
            "ProjectionSnapshotInspectStorage should propagate, got {}",
            classify_failure(other)
        ),
    }

    match super::GitMirrorRunFailure::from_snapshot_bootstrap_error(
        super::GitSnapshotBootstrapError::ProjectionSnapshotInspect {
            message: "duplicate live paths".into(),
        },
    ) {
        super::GitMirrorRunFailure::OutOfSync(reason) => {
            assert!(reason.contains("duplicate live paths"), "{reason}");
        }
        super::GitMirrorRunFailure::Propagate(err) => {
            panic!("ProjectionSnapshotInspect should remain out-of-sync, got {err:?}");
        }
    }
}

fn classify_failure(failure: super::GitMirrorRunFailure) -> String {
    match failure {
        super::GitMirrorRunFailure::OutOfSync(reason) => format!("out_of_sync: {reason}"),
        super::GitMirrorRunFailure::Propagate(err) => format!("propagate: {err:?}"),
    }
}
