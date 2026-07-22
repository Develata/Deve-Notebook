//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Monotonic persistence of owner-issued removal progress.

use super::super::model::RepoLifecycleJobError;
use super::super::removal::{RemovalProgressCommand, RemovalProgressUpdate};
use super::super::store::ReceiptStore;
use super::super::store::removal::{
    RemovalCleanupDisposition, RemovalCleanupReceipt, RemovalCleanupStep, RemovalCutState,
    RemovalTerminalState,
};

pub(super) fn apply(
    store: &mut ReceiptStore,
    command: RemovalProgressCommand,
) -> Result<(), RepoLifecycleJobError> {
    let result = store.update_removal_execution(
        command.preparation_id,
        command.execute_request_id,
        |state, receipt| match command.update {
            RemovalProgressUpdate::SealRemoteImport(plan) => {
                match &state.remote_import_plan {
                    None if matches!(state.cut, RemovalCutState::NotAttempted) => {
                        state.remote_import_checkpoint = Some(plan.initial_checkpoint());
                        state.remote_import_plan = Some(*plan);
                    }
                    Some(current) if current == plan.as_ref() => {}
                    _ => return Err(RepoLifecycleJobError::ConfirmationStale),
                }
                Ok(())
            }
            RemovalProgressUpdate::RemoteImportCheckpoint(checkpoint) => {
                if !matches!(state.cut, RemovalCutState::Observed { .. })
                    || state.remote_import_plan.is_none()
                {
                    return Err(RepoLifecycleJobError::Coordination(
                        "Remote Import cleanup checkpoint preceded owner cut",
                    ));
                }
                if state
                    .remote_import_checkpoint
                    .as_ref()
                    .is_some_and(|current| current.is_complete() && current != checkpoint.as_ref())
                {
                    return Err(RepoLifecycleJobError::ConfirmationStale);
                }
                state.remote_import_checkpoint = Some(*checkpoint);
                Ok(())
            }
            RemovalProgressUpdate::NotegitCheckpoint(checkpoint) => {
                if !matches!(state.cut, RemovalCutState::Observed { .. }) {
                    return Err(RepoLifecycleJobError::Coordination(
                        ".notegit checkpoint preceded owner cut",
                    ));
                }
                if state
                    .notegit_checkpoint
                    .as_ref()
                    .is_some_and(|current| current.is_complete() && current != &checkpoint)
                {
                    return Err(RepoLifecycleJobError::ConfirmationStale);
                }
                state.notegit_checkpoint = Some(checkpoint);
                Ok(())
            }
            RemovalProgressUpdate::AuthorityCheckpoint(checkpoint) => {
                if !matches!(state.cut, RemovalCutState::Observed { .. }) {
                    return Err(RepoLifecycleJobError::Coordination(
                        "authority checkpoint preceded owner cut",
                    ));
                }
                if state
                    .authority_checkpoint
                    .as_ref()
                    .is_some_and(|current| current.is_complete() && current != checkpoint.as_ref())
                {
                    return Err(RepoLifecycleJobError::ConfirmationStale);
                }
                state.authority_checkpoint = Some(*checkpoint);
                Ok(())
            }
            RemovalProgressUpdate::CutAttempted => {
                if state.remote_import_plan.is_none() {
                    return Err(RepoLifecycleJobError::Coordination(
                        "removal cut preceded owner-plan seal",
                    ));
                }
                match &state.cut {
                    RemovalCutState::NotAttempted => state.cut = RemovalCutState::Attempted,
                    RemovalCutState::Attempted => {}
                    RemovalCutState::Observed { .. } => {
                        return Err(RepoLifecycleJobError::ConfirmationStale);
                    }
                }
                Ok(())
            }
            RemovalProgressUpdate::CutObserved(tombstone) => {
                match &state.cut {
                    RemovalCutState::Attempted => {
                        state.cut = RemovalCutState::Observed { tombstone };
                    }
                    RemovalCutState::Observed { tombstone: current } if current == &tombstone => {}
                    _ => return Err(RepoLifecycleJobError::ConfirmationStale),
                }
                Ok(())
            }
            RemovalProgressUpdate::CutNotCommitted => {
                match &state.cut {
                    RemovalCutState::Attempted => state.cut = RemovalCutState::NotAttempted,
                    RemovalCutState::NotAttempted => {}
                    RemovalCutState::Observed { .. } => {
                        return Err(RepoLifecycleJobError::ConfirmationStale);
                    }
                }
                Ok(())
            }
            RemovalProgressUpdate::CleanupStep { step, disposition } => {
                if let Some(existing) = state.cleanup.iter().find(|item| item.step == step) {
                    if existing.disposition == disposition {
                        return Ok(());
                    }
                    if existing.disposition == RemovalCleanupDisposition::Failed
                        && disposition.is_success()
                    {
                        let existing = state
                            .cleanup
                            .iter_mut()
                            .find(|item| item.step == step)
                            .expect("existing cleanup receipt remains addressable");
                        existing.disposition = disposition;
                        existing.completed_at_ms = chrono::Utc::now().timestamp_millis();
                        return Ok(());
                    }
                    return Err(RepoLifecycleJobError::ConfirmationStale);
                }
                if state
                    .cleanup
                    .iter()
                    .any(|receipt| !receipt.disposition.is_success())
                {
                    return Err(RepoLifecycleJobError::Coordination(
                        "removal cleanup cannot advance past a failed owner step",
                    ));
                }
                let expected = RemovalCleanupStep::ORDER
                    .get(state.cleanup.len())
                    .copied()
                    .ok_or(RepoLifecycleJobError::Coordination(
                        "removal cleanup receipt overflow",
                    ))?;
                if step != expected || !matches!(state.cut, RemovalCutState::Observed { .. }) {
                    return Err(RepoLifecycleJobError::Coordination(
                        "removal cleanup step is out of order",
                    ));
                }
                state.cleanup.push(RemovalCleanupReceipt {
                    step,
                    disposition,
                    completed_at_ms: chrono::Utc::now().timestamp_millis(),
                });
                Ok(())
            }
            RemovalProgressUpdate::CleanupComplete => {
                if state.cleanup.len() != RemovalCleanupStep::ORDER.len() {
                    return Err(RepoLifecycleJobError::Coordination(
                        "CleanupComplete preceded owner receipts",
                    ));
                }
                state.cleanup_complete = true;
                Ok(())
            }
            RemovalProgressUpdate::TombstoneRetired => {
                if !state.cleanup_complete {
                    return Err(RepoLifecycleJobError::Coordination(
                        "tombstone retirement preceded CleanupComplete",
                    ));
                }
                state.tombstone_retired = true;
                Ok(())
            }
            RemovalProgressUpdate::TerminalCandidate(completion) => {
                if !state.tombstone_retired {
                    return Err(RepoLifecycleJobError::Coordination(
                        "terminal removal preceded tombstone retirement",
                    ));
                }
                if completion.publication.is_none() {
                    return Err(RepoLifecycleJobError::Coordination(
                        "successful removal candidate has no publication",
                    ));
                }
                match &state.terminal {
                    RemovalTerminalState::None => {
                        state.terminal = RemovalTerminalState::Candidate { completion };
                    }
                    RemovalTerminalState::Candidate {
                        completion: current,
                    } if current == &completion => {}
                    _ => return Err(RepoLifecycleJobError::ConfirmationStale),
                }
                Ok(())
            }
            RemovalProgressUpdate::TerminalComplete => match &state.terminal {
                RemovalTerminalState::Candidate { completion } => {
                    receipt.complete((**completion).clone());
                    state.terminal = RemovalTerminalState::Complete;
                    Ok(())
                }
                RemovalTerminalState::Complete => Ok(()),
                RemovalTerminalState::None => Err(RepoLifecycleJobError::Coordination(
                    "terminal completion preceded TerminalCandidate",
                )),
            },
        },
    );
    match result {
        Ok(state) => {
            let _ = command.reply.send(Ok(state));
            Ok(())
        }
        Err(error) => {
            let fatal_store = match &error {
                RepoLifecycleJobError::Store(detail) => Some(detail.clone()),
                _ => None,
            };
            let _ = command.reply.send(Err(error));
            match fatal_store {
                Some(detail) => Err(RepoLifecycleJobError::Store(detail)),
                None => Ok(()),
            }
        }
    }
}
