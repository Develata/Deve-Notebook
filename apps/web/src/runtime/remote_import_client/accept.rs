//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 07_network#remote-import-wire-contract
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!
//! Exact Remote Import response admission.

use super::{PendingKind, RemoteImportClient};
use crate::runtime::remote_import_client::RemoteImportDiffProjection;
use deve_core::protocol::{
    RemoteImportResponse, RemoteImportResponseContext, RemoteImportSessionView,
};
use leptos::prelude::Update;

pub(super) fn accept(client: &RemoteImportClient, response: RemoteImportResponse) -> bool {
    let context = response_context(&response).clone();
    let Some(current_scope) = client.synchronize_current_scope() else {
        return false;
    };
    let pending = client
        .pending
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&context.request_id);
    client.publish_pending_state();
    let Some(pending) = pending else {
        return false;
    };
    if current_scope.repo_id != context.repo_id
        || current_scope.branch != context.branch
        || current_scope.scope_nonce != context.scope_nonce
        || pending.scope != current_scope
        || !client.selection_accepts(&pending)
        || !pending.kind.accepts(&response, &context)
    {
        return false;
    }

    match response {
        RemoteImportResponse::Prepared { session, .. } => {
            client.install_backend_selection(session.session_id, session.revision);
            client
                .projection
                .update(|projection| projection.install_session(session));
        }
        RemoteImportResponse::Session { session, .. } => {
            if matches!(pending.kind, PendingKind::Refresh { .. }) {
                client.install_backend_selection(session.session_id, session.revision);
            }
            client
                .projection
                .update(|projection| projection.install_session(session));
        }
        RemoteImportResponse::Discarded { session, .. } => {
            client
                .projection
                .update(|projection| projection.install_session(session));
        }
        RemoteImportResponse::Sessions { sessions, .. } => {
            client.projection.update(|projection| {
                projection.sessions = sessions;
                if let Some(selection) = projection.selection {
                    projection.selected_session = projection
                        .sessions
                        .iter()
                        .find(|session| {
                            session.session_id == selection.session_id
                                && session.revision == selection.revision
                        })
                        .cloned()
                        .or_else(|| projection.selected_session.clone());
                }
                projection.error = None;
            });
        }
        RemoteImportResponse::Page { page, .. } => {
            let append = matches!(
                pending.kind,
                PendingKind::Page {
                    cursor: Some(_),
                    ..
                }
            );
            client
                .projection
                .update(|projection| projection.install_page(page, append));
        }
        RemoteImportResponse::Diff {
            entry_id,
            display_label,
            change_kind,
            blockers,
            projection,
            ..
        } => {
            let (session_id, revision) = match pending.kind {
                PendingKind::Diff {
                    session_id,
                    revision,
                    ..
                } => (session_id, revision),
                _ => return false,
            };
            client.projection.update(|state| {
                state.diff = Some(RemoteImportDiffProjection {
                    session_id,
                    revision,
                    entry_id,
                    display_label,
                    change_kind,
                    blockers,
                    projection,
                });
                state.error = None;
            });
        }
        RemoteImportResponse::Applied { receipt, .. } => {
            let session_id = receipt.session_id;
            let revision = receipt.revision;
            client.projection.update(|projection| {
                projection.last_apply = Some(receipt);
                projection.error = None;
            });
            let _ = client.show(session_id, Some(revision));
        }
        RemoteImportResponse::Error { error, .. } => {
            client
                .projection
                .update(|projection| projection.error = Some(error.code));
        }
    }
    true
}

impl PendingKind {
    fn accepts(
        &self,
        response: &RemoteImportResponse,
        context: &RemoteImportResponseContext,
    ) -> bool {
        match (self, response) {
            (Self::Prepare, RemoteImportResponse::Prepared { session, .. }) => {
                session.revision.is_some() && context_matches_session(context, session)
            }
            (Self::List, RemoteImportResponse::Sessions { .. }) => {
                context.session_id.is_none() && context.revision.is_none()
            }
            (
                Self::Show {
                    session_id,
                    revision,
                },
                RemoteImportResponse::Session { session, .. },
            )
            | (
                Self::Discard {
                    session_id,
                    revision,
                },
                RemoteImportResponse::Discarded { session, .. },
            ) => {
                context.session_id == Some(*session_id)
                    && context.revision == *revision
                    && context_matches_session(context, session)
            }
            (
                Self::Page {
                    session_id,
                    revision,
                    ..
                },
                RemoteImportResponse::Page { page, .. },
            ) => {
                context.session_id == Some(*session_id)
                    && context.revision == Some(*revision)
                    && context_matches_session(context, &page.session)
            }
            (
                Self::Diff {
                    session_id,
                    revision,
                    entry_id,
                },
                RemoteImportResponse::Diff {
                    entry_id: actual, ..
                },
            ) => {
                context.session_id == Some(*session_id)
                    && context.revision == Some(*revision)
                    && entry_id == actual
            }
            (
                Self::Refresh {
                    session_id,
                    revision,
                },
                RemoteImportResponse::Session { session, .. },
            ) => {
                context.session_id == Some(*session_id)
                    && context.revision.is_some()
                    && context.revision.is_some_and(|actual| actual > *revision)
                    && context_matches_session(context, session)
            }
            (
                Self::Apply {
                    session_id,
                    revision,
                },
                RemoteImportResponse::Applied { receipt, .. },
            ) => {
                context.session_id == Some(*session_id)
                    && context.revision == Some(*revision)
                    && receipt.request_id == context.request_id
                    && receipt.session_id == *session_id
                    && receipt.revision == *revision
            }
            (_, RemoteImportResponse::Error { .. }) => error_context_matches(self, context),
            _ => false,
        }
    }
}

fn error_context_matches(kind: &PendingKind, context: &RemoteImportResponseContext) -> bool {
    match kind {
        PendingKind::Prepare | PendingKind::List => {
            context.session_id.is_none() && context.revision.is_none()
        }
        PendingKind::Show {
            session_id,
            revision,
        }
        | PendingKind::Discard {
            session_id,
            revision,
        } => context.session_id == Some(*session_id) && context.revision == *revision,
        PendingKind::Page {
            session_id,
            revision,
            ..
        }
        | PendingKind::Refresh {
            session_id,
            revision,
        }
        | PendingKind::Apply {
            session_id,
            revision,
        } => context.session_id == Some(*session_id) && context.revision == Some(*revision),
        PendingKind::Diff {
            session_id,
            revision,
            ..
        } => context.session_id == Some(*session_id) && context.revision == Some(*revision),
    }
}

fn context_matches_session(
    context: &RemoteImportResponseContext,
    session: &RemoteImportSessionView,
) -> bool {
    context.session_id == Some(session.session_id) && context.revision == session.revision
}

fn response_context(response: &RemoteImportResponse) -> &RemoteImportResponseContext {
    match response {
        RemoteImportResponse::Prepared { context, .. }
        | RemoteImportResponse::Sessions { context, .. }
        | RemoteImportResponse::Session { context, .. }
        | RemoteImportResponse::Page { context, .. }
        | RemoteImportResponse::Diff { context, .. }
        | RemoteImportResponse::Applied { context, .. }
        | RemoteImportResponse::Discarded { context, .. }
        | RemoteImportResponse::Error { context, .. } => context,
    }
}
