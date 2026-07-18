//! plan_ref:
//!   - 07_network#remote-import-wire-contract
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!
//! Remote Import WebSocket request dispatch.

use super::response::{
    RemoteImportPhase, core_revision, core_session_id, response_context, run_blocking,
    send_gate_error, send_host_error,
};
use crate::remote_import_wire as wire;
use crate::server::repo_scope::ResolvedRepo;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::{
    RemoteImportRequest, RemoteImportRequestContext, RemoteImportResponse, ServerErrorCode,
    ServerMessage,
};
use std::sync::Arc;

pub(super) async fn dispatch(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: ResolvedRepo,
    context: RemoteImportRequestContext,
    request: RemoteImportRequest,
) {
    let coordinator = state.remote_import_coordinator();
    match request {
        RemoteImportRequest::Prepare { provider, .. } => {
            let result = run_blocking({
                let coordinator = coordinator.clone();
                let repo_name = scope.repo_name;
                move || coordinator.prepare(&repo_name, context.repo_id, provider)
            })
            .await;
            match result {
                Ok(session) => ch.unicast(ServerMessage::RemoteImport(
                    RemoteImportResponse::Prepared {
                        context: response_context(
                            &context,
                            (
                                Some(wire::session_id(session.session_id)),
                                session.revision.map(wire::revision),
                            ),
                        ),
                        session: wire::session(session),
                    },
                )),
                Err(error) => send_host_error(
                    ch,
                    response_context(&context, (None, None)),
                    error,
                    RemoteImportPhase::Prepare,
                ),
            }
        }
        RemoteImportRequest::List { .. } => {
            let result = run_blocking({
                let coordinator = coordinator.clone();
                move || coordinator.list(context.repo_id)
            })
            .await;
            match result {
                Ok(sessions) => ch.unicast(ServerMessage::RemoteImport(
                    RemoteImportResponse::Sessions {
                        context: response_context(&context, (None, None)),
                        sessions: sessions.into_iter().map(wire::session).collect(),
                    },
                )),
                Err(error) => send_host_error(
                    ch,
                    response_context(&context, (None, None)),
                    error,
                    RemoteImportPhase::Read,
                ),
            }
        }
        RemoteImportRequest::Show {
            session_id,
            revision,
            ..
        } => {
            let result = run_blocking({
                let coordinator = coordinator.clone();
                let repo_name = scope.repo_name;
                move || {
                    coordinator.show(
                        &repo_name,
                        context.repo_id,
                        core_session_id(session_id),
                        revision.map(core_revision),
                    )
                }
            })
            .await;
            match result {
                Ok(session)
                    if revision.is_none_or(|expected| {
                        session.revision.map(wire::revision) == Some(expected)
                    }) =>
                {
                    ch.unicast(ServerMessage::RemoteImport(RemoteImportResponse::Session {
                        context: response_context(
                            &context,
                            (Some(session_id), session.revision.map(wire::revision)),
                        ),
                        session: wire::session(session),
                    }))
                }
                Ok(_) => super::response::send_error(
                    ch,
                    response_context(&context, (Some(session_id), revision)),
                    ServerErrorCode::RemoteImportInvalidState,
                ),
                Err(error) => send_host_error(
                    ch,
                    response_context(&context, (Some(session_id), revision)),
                    error,
                    RemoteImportPhase::Read,
                ),
            }
        }
        RemoteImportRequest::Page {
            session_id,
            revision,
            cursor,
            limit,
            ..
        } => {
            let result = run_blocking({
                let coordinator = coordinator.clone();
                let repo_name = scope.repo_name;
                move || {
                    let cursor = cursor
                        .map(|cursor| {
                            deve_core::remote_import::RemoteImportPageCursor::parse(cursor.as_str())
                        })
                        .transpose()?;
                    coordinator.page(
                        &repo_name,
                        context.repo_id,
                        core_session_id(session_id),
                        core_revision(revision),
                        cursor.as_ref(),
                        usize::from(if limit == 0 {
                            deve_core::protocol::remote_import::REMOTE_IMPORT_DEFAULT_PAGE_SIZE
                        } else {
                            limit
                        }),
                    )
                }
            })
            .await;
            match result {
                Ok(page) => ch.unicast(ServerMessage::RemoteImport(RemoteImportResponse::Page {
                    context: response_context(&context, (Some(session_id), Some(revision))),
                    page: wire::page(page),
                })),
                Err(error) => send_host_error(
                    ch,
                    response_context(&context, (Some(session_id), Some(revision))),
                    error,
                    RemoteImportPhase::Read,
                ),
            }
        }
        RemoteImportRequest::Diff {
            session_id,
            revision,
            entry_id,
            ..
        } => {
            let result = run_blocking({
                let coordinator = coordinator.clone();
                let repo_name = scope.repo_name;
                let entry_value = entry_id.as_str().to_string();
                move || {
                    let entry_id =
                        deve_core::remote_import::RemoteImportEntryId::parse(entry_value)?;
                    coordinator.diff(
                        &repo_name,
                        context.repo_id,
                        core_session_id(session_id),
                        core_revision(revision),
                        &entry_id,
                    )
                }
            })
            .await;
            match result {
                Ok(diff) => {
                    let message = ServerMessage::RemoteImport(RemoteImportResponse::Diff {
                        context: response_context(&context, (Some(session_id), Some(revision))),
                        entry_id: wire::entry_id(diff.entry_id),
                        display_label: diff.display_label,
                        change_kind: wire::change_kind(diff.change_kind),
                        blockers: diff.blockers.into_iter().map(wire::blocker).collect(),
                        projection: Arc::new(diff.projection),
                    });
                    let _ = ch.diff_unicast(message).await;
                }
                Err(error) => send_host_error(
                    ch,
                    response_context(&context, (Some(session_id), Some(revision))),
                    error,
                    RemoteImportPhase::Read,
                ),
            }
        }
        RemoteImportRequest::Refresh {
            session_id,
            revision,
            ..
        } => {
            let result = run_blocking({
                let coordinator = coordinator.clone();
                let repo_name = scope.repo_name;
                move || {
                    coordinator.refresh(
                        &repo_name,
                        context.repo_id,
                        core_session_id(session_id),
                        core_revision(revision),
                    )
                }
            })
            .await;
            match result {
                Ok(session) => {
                    ch.unicast(ServerMessage::RemoteImport(RemoteImportResponse::Session {
                        context: response_context(
                            &context,
                            (Some(session_id), session.revision.map(wire::revision)),
                        ),
                        session: wire::session(session),
                    }))
                }
                Err(error) => send_host_error(
                    ch,
                    response_context(&context, (Some(session_id), Some(revision))),
                    error,
                    RemoteImportPhase::Prepare,
                ),
            }
        }
        RemoteImportRequest::Apply {
            session_id,
            revision,
            ..
        } => {
            let exact_replay = run_blocking({
                let coordinator = coordinator.clone();
                move || {
                    coordinator.is_exact_apply_replay(
                        context.repo_id,
                        context.request_id,
                        core_session_id(session_id),
                        core_revision(revision),
                    )
                }
            })
            .await;
            let exact_replay = match exact_replay {
                Ok(exact_replay) => exact_replay,
                Err(error) => {
                    return send_host_error(
                        ch,
                        response_context(&context, (Some(session_id), Some(revision))),
                        error,
                        RemoteImportPhase::Apply,
                    );
                }
            };
            let writer = if exact_replay {
                super::super::local_writer::require_exact_local_writer_identity(session, &scope)
            } else {
                super::super::local_writer::require_exact_local_writer(state, session, &scope)
            };
            if let Err(error) = writer {
                return ch.unicast(ServerMessage::RemoteImport(RemoteImportResponse::Error {
                    context: response_context(&context, (Some(session_id), Some(revision))),
                    error,
                }));
            }
            let gate = state.repo_mutation_gate();
            let repo_name = scope.repo_name;
            let apply = gate
                .execute_mounted_repo_unpublished_blocking(context.repo_id, {
                    let coordinator = coordinator.clone();
                    move || {
                        coordinator.apply(
                            &repo_name,
                            context.repo_id,
                            context.request_id,
                            core_session_id(session_id),
                            core_revision(revision),
                        )
                    }
                })
                .await;
            match apply {
                Ok(Ok(receipt)) => {
                    ch.unicast(ServerMessage::RemoteImport(RemoteImportResponse::Applied {
                        context: response_context(&context, (Some(session_id), Some(revision))),
                        receipt: wire::receipt(receipt),
                    }))
                }
                Ok(Err(error)) => send_host_error(
                    ch,
                    response_context(&context, (Some(session_id), Some(revision))),
                    error,
                    RemoteImportPhase::Apply,
                ),
                Err(error) => send_gate_error(
                    ch,
                    response_context(&context, (Some(session_id), Some(revision))),
                    error,
                ),
            }
        }
        RemoteImportRequest::Discard {
            session_id,
            revision,
            ..
        } => {
            let result = run_blocking({
                let coordinator = coordinator.clone();
                move || {
                    coordinator.discard(
                        context.repo_id,
                        core_session_id(session_id),
                        revision.map(core_revision),
                    )
                }
            })
            .await;
            match result {
                Ok(session) => ch.unicast(ServerMessage::RemoteImport(
                    RemoteImportResponse::Discarded {
                        context: response_context(&context, (Some(session_id), revision)),
                        session: wire::session(session),
                    },
                )),
                Err(error) => send_host_error(
                    ch,
                    response_context(&context, (Some(session_id), revision)),
                    error,
                    RemoteImportPhase::Prepare,
                ),
            }
        }
    }
}
