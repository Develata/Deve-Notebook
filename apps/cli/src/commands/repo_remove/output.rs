//! plan_ref:
//!   - 14_commands#repo-removal-command-contract

use super::token::CliRemovalToken;
use crate::cli_exit::CliProcessExit;
use crate::server::{RepoLifecycleJobError, RepoRemovalRepairInspection};
use anyhow::{Result, anyhow};
use deve_core::models::RepoId;
use deve_core::protocol::{LocalRepoRemovalPreview, RepoLifecycleOutcome, RepoLifecycleState};
use serde::Serialize;
use uuid::Uuid;

pub(super) fn prepared(
    repo_id: RepoId,
    preparation_id: Uuid,
    preview: &LocalRepoRemovalPreview,
    token: Option<CliRemovalToken>,
) -> Result<()> {
    println!("repo_removal=preview");
    println!("repo_id={repo_id}");
    println!("deleted={}", labels(&preview.deleted)?);
    println!("preserved={}", labels(&preview.preserved)?);
    println!("warnings={}", labels(&preview.warnings)?);
    println!("blockers={}", labels(&preview.blockers)?);
    match token {
        Some(token) => println!("confirmation_token={}", token.encode()),
        None => println!("confirmation_token=unavailable"),
    }
    println!("preparation_id={preparation_id}");
    Ok(())
}

pub(super) fn accepted(repo_id: RepoId, request_id: Uuid, job_id: Uuid) {
    println!("repo_removal=accepted");
    println!("repo_id={repo_id}");
    println!("request_id={request_id}");
    println!("job_id={job_id}");
}

pub(super) fn repair_prepared(
    inspection: &RepoRemovalRepairInspection,
    token: Option<&str>,
    expires_at_unix_ms: Option<i64>,
) -> Result<()> {
    println!("repo_removal_repair=preview");
    println!("request_id={}", inspection.request_id);
    println!("repo_id={}", inspection.repo_id);
    println!(
        "remaining={}",
        serde_json::to_string(&inspection.remaining)?
    );
    println!("apply_allowed={}", inspection.apply_allowed);
    println!("repair_token={}", token.unwrap_or("unavailable"));
    println!(
        "expires_at_unix_ms={}",
        expires_at_unix_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unavailable".to_owned())
    );
    Ok(())
}

pub(super) fn repair_accepted(repo_id: RepoId, request_id: Uuid, job_id: Uuid) {
    println!("repo_removal_repair=accepted");
    println!("repo_id={repo_id}");
    println!("request_id={request_id}");
    println!("job_id={job_id}");
}

pub(super) fn terminal(
    repo_id: RepoId,
    request_id: Uuid,
    state: RepoLifecycleState,
    outcome: Option<RepoLifecycleOutcome>,
    publication_pending: bool,
) -> Result<()> {
    println!("repo_removal=terminal");
    println!("repo_id={repo_id}");
    println!("request_id={request_id}");
    println!("state={}", label(&state)?);
    println!(
        "outcome={}",
        outcome
            .as_ref()
            .map(label)
            .transpose()?
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!("publication_pending={publication_pending}");
    if state != RepoLifecycleState::Terminal {
        return Err(exit(22, "REPO_LIFECYCLE_REPAIR_REQUIRED"));
    }
    match outcome {
        Some(RepoLifecycleOutcome::Succeeded) if !publication_pending => Ok(()),
        Some(RepoLifecycleOutcome::NotCommitted) => Err(exit(20, "REPO_LIFECYCLE_NOT_COMMITTED")),
        Some(RepoLifecycleOutcome::CommittedPartial) => {
            Err(exit(21, "REPO_LIFECYCLE_COMMITTED_PARTIAL"))
        }
        Some(RepoLifecycleOutcome::Succeeded) if publication_pending => {
            Err(exit(21, "REPO_LIFECYCLE_PUBLICATION_PENDING"))
        }
        Some(RepoLifecycleOutcome::RepairRequired) | None => {
            Err(exit(22, "REPO_LIFECYCLE_REPAIR_REQUIRED"))
        }
        Some(RepoLifecycleOutcome::Succeeded) => unreachable!("guarded above"),
    }
}

pub(super) fn symbolic_error(code: String) -> anyhow::Error {
    match code.as_str() {
        "REPO_LIFECYCLE_REPAIR_REQUIRED" => exit(22, "REPO_LIFECYCLE_REPAIR_REQUIRED"),
        _ => anyhow!(code),
    }
}

pub(super) fn sanitize(error: anyhow::Error) -> anyhow::Error {
    if error
        .chain()
        .any(|cause| cause.downcast_ref::<CliProcessExit>().is_some())
    {
        return error;
    }
    if let Some(lifecycle) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<RepoLifecycleJobError>())
    {
        return match lifecycle {
            RepoLifecycleJobError::RemovalRepairBlocked => {
                exit(22, "REPO_LIFECYCLE_REPAIR_REQUIRED")
            }
            RepoLifecycleJobError::OwnerActive | RepoLifecycleJobError::Busy => {
                anyhow!("REPO_LIFECYCLE_BUSY")
            }
            RepoLifecycleJobError::RemovalRepairNotRequired => {
                anyhow!("REPO_LIFECYCLE_INVALID_REQUEST")
            }
            RepoLifecycleJobError::NotFound => anyhow!("REPO_LIFECYCLE_NOT_FOUND"),
            RepoLifecycleJobError::ConfirmationInvalid => {
                anyhow!("REPO_LIFECYCLE_CONFIRMATION_INVALID")
            }
            RepoLifecycleJobError::ConfirmationExpired => {
                anyhow!("REPO_LIFECYCLE_CONFIRMATION_EXPIRED")
            }
            RepoLifecycleJobError::ConfirmationStale => {
                anyhow!("REPO_LIFECYCLE_CONFIRMATION_STALE")
            }
            RepoLifecycleJobError::InvalidRequest | RepoLifecycleJobError::RequestConflict => {
                anyhow!("REPO_LIFECYCLE_INVALID_REQUEST")
            }
            RepoLifecycleJobError::RemovalBlocked => anyhow!("REPO_LIFECYCLE_REMOVAL_BLOCKED"),
            RepoLifecycleJobError::AdmissionClosed
            | RepoLifecycleJobError::Store(_)
            | RepoLifecycleJobError::Coordination(_)
            | RepoLifecycleJobError::Shutdown(_) => exit(22, "REPO_LIFECYCLE_REPAIR_REQUIRED"),
        };
    }
    let label = error.to_string();
    if is_symbolic_label(&label) {
        return symbolic_error(label);
    }
    anyhow!("REPO_LIFECYCLE_REPAIR_REQUIRED")
}

fn is_symbolic_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn labels<T: Serialize>(values: &[T]) -> Result<String> {
    values
        .iter()
        .map(label)
        .collect::<Result<Vec<_>>>()
        .map(|v| v.join(","))
}

fn label<T: Serialize>(value: &T) -> Result<String> {
    let json = serde_json::to_string(value)?;
    Ok(json.trim_matches('"').to_string())
}

fn exit(code: u8, label: &'static str) -> anyhow::Error {
    anyhow::Error::new(CliProcessExit::new(code, label))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_exit_code;
    use std::process::Command;

    const PROCESS_FIXTURE_ENV: &str = "DEVE_TEST_REMOVAL_PROCESS_OUTCOME";
    const PROCESS_TEST_NAME: &str =
        "commands::repo_remove::output::tests::lifecycle_outcomes_cross_process_boundary";

    #[test]
    fn lifecycle_outcomes_map_to_contract_exit_codes() {
        assert_eq!(
            process_exit_code(
                &terminal(
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    RepoLifecycleState::Terminal,
                    Some(RepoLifecycleOutcome::NotCommitted),
                    false,
                )
                .expect_err("not committed")
            ),
            20
        );
        assert_eq!(
            process_exit_code(
                &terminal(
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    RepoLifecycleState::Terminal,
                    Some(RepoLifecycleOutcome::CommittedPartial),
                    false,
                )
                .expect_err("partial")
            ),
            21
        );
        assert_eq!(
            process_exit_code(
                &terminal(
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    RepoLifecycleState::Terminal,
                    Some(RepoLifecycleOutcome::RepairRequired),
                    false,
                )
                .expect_err("repair")
            ),
            22
        );
    }

    #[test]
    fn lifecycle_outcomes_cross_process_boundary() {
        if let Some(outcome) = std::env::var_os(PROCESS_FIXTURE_ENV) {
            let (outcome, publication_pending) = match outcome.to_string_lossy().as_ref() {
                "not_committed" => (RepoLifecycleOutcome::NotCommitted, false),
                "committed_partial" => (RepoLifecycleOutcome::CommittedPartial, false),
                "publication_pending" => (RepoLifecycleOutcome::Succeeded, true),
                "repair_required" => (RepoLifecycleOutcome::RepairRequired, false),
                other => panic!("unknown process fixture outcome: {other}"),
            };
            let error = terminal(
                Uuid::new_v4(),
                Uuid::new_v4(),
                RepoLifecycleState::Terminal,
                Some(outcome),
                publication_pending,
            )
            .expect_err("fixture outcome must be non-zero");
            std::process::exit(i32::from(process_exit_code(&error)));
        }

        for (outcome, expected) in [
            ("not_committed", 20),
            ("committed_partial", 21),
            ("publication_pending", 21),
            ("repair_required", 22),
        ] {
            let child = Command::new(std::env::current_exe().expect("current test executable"))
                .args(["--exact", PROCESS_TEST_NAME, "--nocapture"])
                .env(PROCESS_FIXTURE_ENV, outcome)
                .output()
                .expect("spawn lifecycle exit fixture");
            assert_eq!(
                child.status.code(),
                Some(expected),
                "{outcome} child failed:\nstdout={}\nstderr={}",
                String::from_utf8_lossy(&child.stdout),
                String::from_utf8_lossy(&child.stderr),
            );
        }
    }

    #[test]
    fn removal_shell_sanitizes_owner_detail_and_maps_blocked_repair_to_22() {
        let blocked = sanitize(anyhow::Error::new(
            RepoLifecycleJobError::RemovalRepairBlocked,
        ));
        assert_eq!(process_exit_code(&blocked), 22);
        assert_eq!(blocked.to_string(), "REPO_LIFECYCLE_REPAIR_REQUIRED");

        let raw = sanitize(anyhow!(
            "failed to read C:\\private\\ledger\\.host\\main_port: secret detail"
        ));
        assert_eq!(process_exit_code(&raw), 1);
        assert_eq!(raw.to_string(), "REPO_LIFECYCLE_REPAIR_REQUIRED");
        assert!(!raw.to_string().contains("private"));
    }
}
