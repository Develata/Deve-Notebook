//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#remote-import-command-contract
//!
//! Remote Import intent dispatch for the authenticated Local CLI proxy.

use super::super::{RemoteImportPhase, run_blocking};
use super::identity::{core_revision, core_session_id, request_identity, response_context};
use super::response::{gate_error, host_error, intent_error, intent_response};
use crate::remote_import_wire as wire;
use crate::server::AppState;
use axum::response::Response;
use deve_core::protocol::{
    RemoteImportRequest, RemoteImportResponse, ServerError, ServerErrorCode,
};
use std::sync::Arc;

pub(super) async fn execute_intent(
    state: Arc<AppState>,
    repo_name: String,
    request: RemoteImportRequest,
) -> Response {
    let context = request.context().clone();
    if context.branch.is_some() || context.scope_nonce.get() == 0 {
        return intent_error(
            response_context(&context, request_identity(&request)),
            ServerErrorCode::RemoteImportInvalidState,
        );
    }
    let coordinator = state.remote_import_coordinator();
    let response = match request {
        RemoteImportRequest::Prepare { provider, .. } => {
            match run_blocking({
                let coordinator = coordinator.clone();
                move || coordinator.prepare(&repo_name, context.repo_id, provider)
            })
            .await
            {
                Ok(session) => RemoteImportResponse::Prepared {
                    context: response_context(
                        &context,
                        (
                            Some(wire::session_id(session.session_id)),
                            session.revision.map(wire::revision),
                        ),
                    ),
                    session: wire::session(session),
                },
                Err(error) => host_error(
                    response_context(&context, (None, None)),
                    error,
                    RemoteImportPhase::Prepare,
                ),
            }
        }
        RemoteImportRequest::List { .. } => match run_blocking({
            let coordinator = coordinator.clone();
            move || coordinator.list(context.repo_id)
        })
        .await
        {
            Ok(sessions) => RemoteImportResponse::Sessions {
                context: response_context(&context, (None, None)),
                sessions: sessions.into_iter().map(wire::session).collect(),
            },
            Err(error) => host_error(
                response_context(&context, (None, None)),
                error,
                RemoteImportPhase::Read,
            ),
        },
        RemoteImportRequest::Show {
            session_id,
            revision,
            ..
        } => match run_blocking({
            let coordinator = coordinator.clone();
            move || {
                coordinator.show(
                    &repo_name,
                    context.repo_id,
                    core_session_id(session_id),
                    revision.map(core_revision),
                )
            }
        })
        .await
        {
            Ok(session) => RemoteImportResponse::Session {
                context: response_context(
                    &context,
                    (Some(session_id), session.revision.map(wire::revision)),
                ),
                session: wire::session(session),
            },
            Err(error) => host_error(
                response_context(&context, (Some(session_id), revision)),
                error,
                RemoteImportPhase::Read,
            ),
        },
        RemoteImportRequest::Page {
            session_id,
            revision,
            cursor,
            limit,
            ..
        } => match run_blocking({
            let coordinator = coordinator.clone();
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
        .await
        {
            Ok(page) => RemoteImportResponse::Page {
                context: response_context(&context, (Some(session_id), Some(revision))),
                page: wire::page(page),
            },
            Err(error) => host_error(
                response_context(&context, (Some(session_id), Some(revision))),
                error,
                RemoteImportPhase::Read,
            ),
        },
        RemoteImportRequest::Diff {
            session_id,
            revision,
            entry_id,
            ..
        } => match run_blocking({
            let coordinator = coordinator.clone();
            let entry = entry_id.as_str().to_string();
            move || {
                let entry = deve_core::remote_import::RemoteImportEntryId::parse(entry)?;
                coordinator.diff(
                    &repo_name,
                    context.repo_id,
                    core_session_id(session_id),
                    core_revision(revision),
                    &entry,
                )
            }
        })
        .await
        {
            Ok(diff) => RemoteImportResponse::Diff {
                context: response_context(&context, (Some(session_id), Some(revision))),
                entry_id: wire::entry_id(diff.entry_id),
                display_label: diff.display_label,
                change_kind: wire::change_kind(diff.change_kind),
                blockers: diff.blockers.into_iter().map(wire::blocker).collect(),
                projection: Arc::new(diff.projection),
            },
            Err(error) => host_error(
                response_context(&context, (Some(session_id), Some(revision))),
                error,
                RemoteImportPhase::Read,
            ),
        },
        RemoteImportRequest::Refresh {
            session_id,
            revision,
            ..
        } => match run_blocking({
            let coordinator = coordinator.clone();
            move || {
                coordinator.refresh(
                    &repo_name,
                    context.repo_id,
                    core_session_id(session_id),
                    core_revision(revision),
                )
            }
        })
        .await
        {
            Ok(session) => RemoteImportResponse::Session {
                context: response_context(
                    &context,
                    (Some(session_id), session.revision.map(wire::revision)),
                ),
                session: wire::session(session),
            },
            Err(error) => host_error(
                response_context(&context, (Some(session_id), Some(revision))),
                error,
                RemoteImportPhase::Prepare,
            ),
        },
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
                    return intent_response(host_error(
                        response_context(&context, (Some(session_id), Some(revision))),
                        error,
                        RemoteImportPhase::Apply,
                    ));
                }
            };
            if !exact_replay
                && let Err(error) = crate::server::repo_scope::ensure_local_repo_projection_writable(
                    &state, &repo_name,
                )
            {
                RemoteImportResponse::Error {
                    context: response_context(&context, (Some(session_id), Some(revision))),
                    error: ServerError::new(error.code),
                }
            } else {
                match state
                    .repo_mutation_gate()
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
                    .await
                {
                    Ok(Ok(receipt)) => RemoteImportResponse::Applied {
                        context: response_context(&context, (Some(session_id), Some(revision))),
                        receipt: wire::receipt(receipt),
                    },
                    Ok(Err(error)) => host_error(
                        response_context(&context, (Some(session_id), Some(revision))),
                        error,
                        RemoteImportPhase::Apply,
                    ),
                    Err(error) => gate_error(
                        response_context(&context, (Some(session_id), Some(revision))),
                        error,
                    ),
                }
            }
        }
        RemoteImportRequest::Discard {
            session_id,
            revision,
            ..
        } => match run_blocking({
            let coordinator = coordinator.clone();
            move || {
                coordinator.discard(
                    context.repo_id,
                    core_session_id(session_id),
                    revision.map(core_revision),
                )
            }
        })
        .await
        {
            Ok(session) => RemoteImportResponse::Discarded {
                context: response_context(&context, (Some(session_id), revision)),
                session: wire::session(session),
            },
            Err(error) => host_error(
                response_context(&context, (Some(session_id), revision)),
                error,
                RemoteImportPhase::Prepare,
            ),
        },
    };
    intent_response(response)
}
