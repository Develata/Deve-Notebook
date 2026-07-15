//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Narrow CLI-owned adapter for managed-note plugin mutation intents. Rhai
//! execution finishes capability/path validation before entering this adapter;
//! only the authority mutation and projection writeback hold the repo permit.

use super::{MutationExecution, MutationPublication};
use crate::server::AppState;
use anyhow::{Context, Result};
use deve_core::ledger::{range, reconcile};
use deve_core::models::{DocId, Op};
use deve_core::plugin::runtime::host::{ManagedNoteMutationHost, ManagedNoteWriteIntent};
use deve_core::plugin::runtime::host::{
    ManagedSourceControlCommitIntent, ManagedSourceControlMutationHost,
};
use deve_core::source_control::{CommitAuthorityFailure, CommitInfo};
use std::sync::{Arc, Weak};

#[cfg_attr(test, allow(dead_code))]
pub(crate) struct CliManagedNoteMutationHost {
    state: Weak<AppState>,
}

#[cfg_attr(test, allow(dead_code))]
impl CliManagedNoteMutationHost {
    pub(crate) fn new(state: &Arc<AppState>) -> Self {
        Self {
            state: Arc::downgrade(state),
        }
    }
}

impl ManagedNoteMutationHost for CliManagedNoteMutationHost {
    fn write_managed_note(&self, intent: ManagedNoteWriteIntent) -> Result<()> {
        let state = self
            .state
            .upgrade()
            .context("managed-note server state is unavailable")?;
        let repo_id = state
            .repo
            .get_repo_info_for(None, Some(&intent.repo_name))?
            .map(|info| info.uuid)
            .context("managed-note repo metadata missing")?;
        let prepared = prepare_managed_note_write(&state, &intent, repo_id)?;
        let runtime = tokio::runtime::Handle::try_current()
            .context("managed-note mutation requires the server Tokio runtime")?;
        let gate = state.repo_mutation_gate();
        let tx = state.tx.clone();
        require_multithread_runtime(&runtime)?;
        let execution = tokio::task::block_in_place(|| {
            runtime.block_on(gate.execute_repo(repo_id, &tx, || {
            let repo_name = match super::revalidate_writable_local_repo(
                &state,
                repo_id,
                &intent.repo_name,
            ) {
                Ok(repo_name) => repo_name,
                Err(error) => return MutationExecution::not_committed(error),
            };
            let current_head = match state
                .repo
                .run_on_local_repo(&repo_name, range::get_max_seq)
            {
                Ok(head) => head,
                Err(error) => return MutationExecution::not_committed(error),
            };
            if current_head != prepared.expected_ledger_head {
                return MutationExecution::not_committed(anyhow::anyhow!(
                    "managed-note authority changed while waiting for mutation permit: expected head {}, observed {}",
                    prepared.expected_ledger_head,
                    current_head
                ));
            }
            match state
                .repo
                .get_tracked_docid_in_local_repo(&repo_name, &intent.repo_path)
            {
                Ok(current) if current == prepared.existing_doc_id => {}
                Ok(_) => {
                    return MutationExecution::not_committed(anyhow::anyhow!(
                        "managed-note path identity changed while waiting for mutation permit"
                    ));
                }
                Err(error) => return MutationExecution::not_committed(error),
            }

            let (doc_id, structure_ops) = match state.repo.apply_file_structure_in_local_repo(
                &repo_name,
                &intent.repo_path,
                None,
                "plugin",
            ) {
                Ok(value) => value,
                Err(error) => return MutationExecution::not_committed(error),
            };
            let structure_committed = !structure_ops.is_empty();
            let publication = MutationPublication::plugin_recovery(repo_id, doc_id);
            if let Some(expected_doc_id) = prepared.existing_doc_id
                && doc_id != expected_doc_id
            {
                return MutationExecution::committed_partial(
                    anyhow::anyhow!("managed-note DocId changed during structure append"),
                    publication,
                );
            }
            let content_will_commit = !prepared.patch.is_empty();
            if content_will_commit
                && let Err(error) = reconcile::append_patch_in_local_repo(
                    state.repo.as_ref(),
                    &repo_name,
                    doc_id,
                    "plugin",
                    &prepared.patch,
                )
            {
                // append_patch may have committed a strict prefix across its
                // per-fact transactions. Any non-empty patch therefore has a
                // committed-partial failure boundary.
                return MutationExecution::committed_partial(error, publication);
            }
            match state
                .sync_manager
                .persist_doc_in_local_repo(&repo_name, doc_id)
            {
                Ok(_) => MutationExecution::committed((), publication),
                Err(error) if structure_committed || content_will_commit => {
                    MutationExecution::projection_degraded((), error, publication)
                }
                Err(error) => MutationExecution::not_committed(error),
            }
        }))
        })
        .map_err(anyhow::Error::new)?;

        match execution {
            MutationExecution::Committed { .. } => Ok(()),
            MutationExecution::NotCommitted(error)
            | MutationExecution::ProjectionDegraded { error, .. }
            | MutationExecution::CommittedPartial { error, .. } => Err(error),
        }
    }
}

struct PreparedManagedNoteWrite {
    existing_doc_id: Option<DocId>,
    expected_ledger_head: u64,
    patch: Vec<Op>,
}

fn prepare_managed_note_write(
    state: &Arc<AppState>,
    intent: &ManagedNoteWriteIntent,
    repo_id: deve_core::models::RepoId,
) -> Result<PreparedManagedNoteWrite> {
    let repo_name = state
        .repo
        .resolve_local_repo_name_for_execution(Some(repo_id), Some(&intent.repo_name))?;
    let existing_doc_id = state
        .repo
        .get_tracked_docid_in_local_repo(&repo_name, &intent.repo_path)?;
    let entries = match existing_doc_id {
        Some(doc_id) => state
            .repo
            .get_local_ops_in_local_repo(&repo_name, doc_id)?
            .into_iter()
            .map(|(_, entry)| entry)
            .collect(),
        None => Vec::new(),
    };
    let patch = reconcile::compute_reconcile_patch(&entries, &intent.content)?;
    let expected_ledger_head = state
        .repo
        .run_on_local_repo(&repo_name, range::get_max_seq)?;
    Ok(PreparedManagedNoteWrite {
        existing_doc_id,
        expected_ledger_head,
        patch,
    })
}

fn require_multithread_runtime(runtime: &tokio::runtime::Handle) -> Result<()> {
    if runtime.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
        anyhow::bail!("managed-note mutation requires a multi-thread Tokio server runtime");
    }
    Ok(())
}

#[cfg_attr(test, allow(dead_code))]
pub(crate) struct CliManagedSourceControlMutationHost {
    state: Weak<AppState>,
}

#[cfg_attr(test, allow(dead_code))]
impl CliManagedSourceControlMutationHost {
    pub(crate) fn new(state: &Arc<AppState>) -> Self {
        Self {
            state: Arc::downgrade(state),
        }
    }
}

impl ManagedSourceControlMutationHost for CliManagedSourceControlMutationHost {
    fn commit_source_control(
        &self,
        intent: ManagedSourceControlCommitIntent,
    ) -> Result<CommitInfo> {
        let state = self
            .state
            .upgrade()
            .context("managed source-control server state is unavailable")?;
        let repo_name = state.repo.resolve_local_repo_name_for_execution(
            intent.selector.repo_id,
            intent.selector.repo_name.as_deref(),
        )?;
        let repo_id = state
            .repo
            .get_repo_info_for(None, Some(&repo_name))?
            .map(|info| info.uuid)
            .context("managed source-control repo metadata missing")?;
        let runtime = tokio::runtime::Handle::try_current()
            .context("managed source-control mutation requires the server Tokio runtime")?;
        let prepared_external = state
            .repo
            .prepare_source_control_commit_in_local_repo(&repo_name)?;
        require_multithread_runtime(&runtime)?;
        let gate = state.repo_mutation_gate();
        let tx = state.tx.clone();
        let execution = tokio::task::block_in_place(|| {
            runtime.block_on(gate.execute_repo(repo_id, &tx, || {
                let bound_name =
                    match super::revalidate_writable_local_repo(&state, repo_id, &repo_name) {
                        Ok(name) => name,
                        Err(error) => return MutationExecution::not_committed(error),
                    };
                match state
                    .repo
                    .commit_source_control_authority_with_prepared_in_local_repo(
                        &bound_name,
                        &intent.message,
                        prepared_external,
                    ) {
                    Ok(info) => MutationExecution::committed(
                        info.clone(),
                        MutationPublication::SourceControlCommit {
                            repo_id,
                            branch: None,
                            scope_nonce: None,
                            commit_id: info.id,
                            timestamp: info.timestamp,
                            recovery: MutationPublication::source_control_recovery(repo_id),
                        },
                    ),
                    Err(CommitAuthorityFailure::NotCommitted(error)) => {
                        MutationExecution::not_committed(error)
                    }
                    Err(CommitAuthorityFailure::CommittedPartial {
                        external_apply,
                        error,
                    }) => MutationExecution::committed_partial(
                        error,
                        MutationPublication::external_apply_recovery(
                            external_apply.repo_id,
                            external_apply.affected_docs,
                        ),
                    ),
                }
            }))
        })
        .map_err(anyhow::Error::new)?;

        match execution {
            MutationExecution::Committed { value: info, .. } => {
                state
                    .repo
                    .enqueue_git_mirror_projection_in_local_repo(&repo_name, repo_id, &info);
                Ok(info)
            }
            MutationExecution::NotCommitted(error)
            | MutationExecution::ProjectionDegraded { error, .. }
            | MutationExecution::CommittedPartial { error, .. } => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::require_multithread_runtime;

    #[test]
    fn current_thread_runtime_fails_closed_before_block_in_place() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let error = require_multithread_runtime(runtime.handle()).expect_err("must reject");
        assert!(error.to_string().contains("multi-thread Tokio"));
    }

    #[test]
    fn multi_thread_server_runtime_is_accepted() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("runtime");
        require_multithread_runtime(runtime.handle()).expect("server runtime");
    }
}
