//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 06_backup#projection-backup-command-output-contract
//!   - 14_commands#remote-import-command-contract

use super::{RemoteImportAction, output};
use crate::local_cli_proxy_contract::LocalCliRemoteImportResponse;
use crate::remote_import_runtime::{RemoteImportCoordinator, RemoteImportHostError};
use crate::remote_import_wire as wire;
use crate::server::handlers::remote_import::{RemoteImportPhase, host_error_code};
use anyhow::{Result, anyhow, bail};
use deve_core::ledger::RepoManager;
use deve_core::protocol::{
    RemoteImportResponse, RemoteImportResponseContext, ScopeNonce, ServerError, ServerErrorCode,
};
use deve_core::remote_import::{
    RemoteImportCandidateRevision, RemoteImportEntryId, RemoteImportSessionId,
};
use deve_core::sync::SyncManager;
use deve_core::sync::watcher::{RepoWatcherHandle, RepoWatcherStart, RepoWatcherWorkerState};
use std::sync::Arc;
use uuid::Uuid;

pub(super) fn run(repo: Arc<RepoManager>, action: RemoteImportAction) -> Result<()> {
    let repo_id = action.repo_id();
    let repo_name = repo.resolve_local_repo_name_for_execution(Some(repo_id), None)?;
    let sync = Arc::new(SyncManager::new_checked(repo.clone())?);
    let membership = repo.catalog_membership_runtime();
    repo.seed_catalog_membership_from_records()?;
    deve_core::remote_import::RemoteImportService::recover_startup(&repo, repo_id)?;
    let coordinator = RemoteImportCoordinator::new(repo.clone(), sync.clone(), membership);
    match action {
        RemoteImportAction::Prepare { provider, .. } => {
            let request_id = Uuid::new_v4();
            let session = safe_host(
                coordinator.prepare(&repo_name, repo_id, provider.into()),
                RemoteImportPhase::Prepare,
            )?;
            emit_intent(RemoteImportResponse::Prepared {
                context: response_context(
                    request_id,
                    repo_id,
                    Some(wire::session_id(session.session_id)),
                    session.revision.map(wire::revision),
                ),
                session: wire::session(session),
            })
        }
        RemoteImportAction::List { .. } => {
            let request_id = Uuid::new_v4();
            let sessions = safe_host(coordinator.list(repo_id), RemoteImportPhase::Read)?;
            emit_intent(RemoteImportResponse::Sessions {
                context: response_context(request_id, repo_id, None, None),
                sessions: sessions.into_iter().map(wire::session).collect(),
            })
        }
        RemoteImportAction::Show {
            session, revision, ..
        } => {
            let request_id = Uuid::new_v4();
            let session_id = RemoteImportSessionId::from_uuid(session);
            match revision {
                Some(revision) => {
                    let revision = RemoteImportCandidateRevision::from_u64(revision);
                    let page = safe_host(
                        coordinator.page(
                            &repo_name,
                            repo_id,
                            session_id,
                            revision,
                            None,
                            deve_core::remote_import::REMOTE_IMPORT_DEFAULT_PAGE_SIZE,
                        ),
                        RemoteImportPhase::Read,
                    )?;
                    emit_intent(RemoteImportResponse::Page {
                        context: response_context(
                            request_id,
                            repo_id,
                            Some(wire::session_id(session_id)),
                            Some(wire::revision(revision)),
                        ),
                        page: wire::page(page),
                    })
                }
                None => {
                    let session = safe_host(
                        coordinator.show(&repo_name, repo_id, session_id, None),
                        RemoteImportPhase::Read,
                    )?;
                    emit_intent(RemoteImportResponse::Session {
                        context: response_context(
                            request_id,
                            repo_id,
                            Some(wire::session_id(session.session_id)),
                            session.revision.map(wire::revision),
                        ),
                        session: wire::session(session),
                    })
                }
            }
        }
        RemoteImportAction::Diff {
            session,
            revision,
            entry,
            ..
        } => {
            let request_id = Uuid::new_v4();
            let session_id = RemoteImportSessionId::from_uuid(session);
            let revision = RemoteImportCandidateRevision::from_u64(revision);
            let entry_id = RemoteImportEntryId::parse(entry)
                .map_err(|_| anyhow!("REMOTE_IMPORT_INVALID_STATE"))?;
            let diff = safe_host(
                coordinator.diff(&repo_name, repo_id, session_id, revision, &entry_id),
                RemoteImportPhase::Read,
            )?;
            emit_intent(RemoteImportResponse::Diff {
                context: response_context(
                    request_id,
                    repo_id,
                    Some(wire::session_id(session_id)),
                    Some(wire::revision(revision)),
                ),
                entry_id: wire::entry_id(diff.entry_id),
                display_label: diff.display_label,
                change_kind: wire::change_kind(diff.change_kind),
                blockers: diff.blockers.into_iter().map(wire::blocker).collect(),
                projection: Arc::new(diff.projection),
            })
        }
        RemoteImportAction::Refresh {
            session, revision, ..
        } => {
            let request_id = Uuid::new_v4();
            let session_id = RemoteImportSessionId::from_uuid(session);
            let revision = RemoteImportCandidateRevision::from_u64(revision);
            let session = safe_host(
                coordinator.refresh(&repo_name, repo_id, session_id, revision),
                RemoteImportPhase::Prepare,
            )?;
            emit_intent(RemoteImportResponse::Session {
                context: response_context(
                    request_id,
                    repo_id,
                    Some(wire::session_id(session.session_id)),
                    session.revision.map(wire::revision),
                ),
                session: wire::session(session),
            })
        }
        RemoteImportAction::Apply {
            session,
            revision,
            request_id,
            ..
        } => {
            let request_id = request_id.unwrap_or_else(Uuid::new_v4);
            eprintln!("remote_import_apply_request_id={request_id}");
            let session_id = RemoteImportSessionId::from_uuid(session);
            let revision = RemoteImportCandidateRevision::from_u64(revision);
            let exact_replay = safe_host(
                coordinator.is_exact_apply_replay(repo_id, request_id, session_id, revision),
                RemoteImportPhase::Apply,
            )?;
            let watcher = start_apply_watcher(sync, &repo_name, exact_replay)?;
            let apply = coordinator.apply(&repo_name, repo_id, request_id, session_id, revision);
            let shutdown = watcher.shutdown();
            match apply {
                Ok(receipt) => {
                    emit_intent(RemoteImportResponse::Applied {
                        context: response_context(
                            request_id,
                            repo_id,
                            Some(wire::session_id(session_id)),
                            Some(wire::revision(revision)),
                        ),
                        receipt: wire::receipt(receipt),
                    })?;
                    shutdown.map_err(anyhow::Error::new)
                }
                Err(error) => {
                    let code = host_error_code(&error, RemoteImportPhase::Apply);
                    emit_error(request_id, repo_id, session_id, revision, code)?;
                    let primary = anyhow!(output::code_name(code));
                    match shutdown {
                        Ok(()) => Err(primary),
                        Err(cleanup) => Err(primary
                            .context(format!("temporary watcher shutdown also failed: {cleanup}"))),
                    }
                }
            }
        }
        RemoteImportAction::Discard {
            session, revision, ..
        } => {
            let request_id = Uuid::new_v4();
            let session_id = RemoteImportSessionId::from_uuid(session);
            let revision = revision.map(RemoteImportCandidateRevision::from_u64);
            let session = safe_host(
                coordinator.discard(repo_id, session_id, revision),
                RemoteImportPhase::Prepare,
            )?;
            emit_intent(RemoteImportResponse::Discarded {
                context: response_context(
                    request_id,
                    repo_id,
                    Some(wire::session_id(session_id)),
                    revision.map(wire::revision),
                ),
                session: wire::session(session),
            })
        }
        RemoteImportAction::Repair { apply, .. } => {
            let request_id = Uuid::new_v4();
            let plan = safe_host(
                coordinator.inspect_repair(repo_id),
                RemoteImportPhase::Prepare,
            )?;
            let plan = if apply {
                safe_host(
                    coordinator.apply_repair(repo_id, plan.token()),
                    RemoteImportPhase::Prepare,
                )?
            } else {
                plan
            };
            let response = LocalCliRemoteImportResponse::Repair {
                request_id,
                repo_id,
                branch: None,
                scope_nonce: ScopeNonce::new(1),
                finding_count: plan.finding_count,
                repairable_count: plan.repairable_count,
            };
            output::print(&response)
        }
    }
}

fn start_apply_watcher(
    sync: Arc<SyncManager>,
    repo_name: &str,
    allow_exact_replay: bool,
) -> Result<RepoWatcherHandle> {
    if !allow_exact_replay
        && !sync
            .healthy_local_repo_names_for_execution()?
            .iter()
            .any(|healthy| healthy == repo_name)
    {
        bail!("REMOTE_IMPORT_INVALID_STATE");
    }
    let handle = RepoWatcherHandle::start(RepoWatcherStart::resolve(sync, repo_name, 1)?)?;
    let failure = match handle.snapshot().worker_state() {
        RepoWatcherWorkerState::Running => return Ok(handle),
        RepoWatcherWorkerState::Failed(failure) => failure.clone(),
    };
    match handle.shutdown() {
        Ok(()) => Err(anyhow::Error::new(failure)),
        Err(cleanup) => Err(anyhow::Error::new(failure)
            .context(format!("temporary watcher shutdown also failed: {cleanup}"))),
    }
}

fn safe_host<T>(result: Result<T, RemoteImportHostError>, phase: RemoteImportPhase) -> Result<T> {
    result.map_err(|error| {
        tracing::warn!(%error, "Remote Import CLI operation failed");
        anyhow!(output::code_name(host_error_code(&error, phase)))
    })
}

fn emit_intent(response: RemoteImportResponse) -> Result<()> {
    let response = LocalCliRemoteImportResponse::Intent { response };
    output::print(&response)?;
    output::ensure_success(&response)
}

fn emit_error(
    request_id: Uuid,
    repo_id: deve_core::models::RepoId,
    session_id: RemoteImportSessionId,
    revision: RemoteImportCandidateRevision,
    code: ServerErrorCode,
) -> Result<()> {
    let response = RemoteImportResponse::Error {
        context: response_context(
            request_id,
            repo_id,
            Some(wire::session_id(session_id)),
            Some(wire::revision(revision)),
        ),
        error: ServerError::new(code),
    };
    output::print(&LocalCliRemoteImportResponse::Intent { response })
}

fn response_context(
    request_id: Uuid,
    repo_id: deve_core::models::RepoId,
    session_id: Option<deve_core::protocol::RemoteImportSessionId>,
    revision: Option<deve_core::protocol::RemoteImportCandidateRevision>,
) -> RemoteImportResponseContext {
    RemoteImportResponseContext {
        request_id,
        repo_id,
        branch: None,
        scope_nonce: ScopeNonce::new(1),
        session_id,
        revision,
    }
}
