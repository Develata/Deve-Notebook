//! plan_ref:
//!   - 09_web_thin_client_ledger#document-create-intent
//!   - 07_network#projection-recovery-contract
//!
//! Hooks adapter for typed Document Create confirmation.

use crate::hooks::use_core::sync_banner_notice::show_temporary_sync_banner;
use crate::i18n::Locale;
use crate::runtime::document::create::CreateResponseDisposition;
use deve_core::protocol::DocumentCreateResponse;
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::state::CoreSignals;
use super::message_projection::settle_pending_document_create;

pub(super) fn handle(response: DocumentCreateResponse, locale: Locale, signals: CoreSignals) {
    let mut disposition = CreateResponseDisposition::Ignored;
    signals.set_pending_document_create.update(|pending| {
        if let Some(pending) = pending.as_mut() {
            disposition = pending.accept_response(&response);
        }
    });
    match disposition {
        CreateResponseDisposition::Ignored => {}
        CreateResponseDisposition::WaitingForProjection => {
            let docs = signals.docs.get_untracked();
            settle_pending_document_create(&docs, signals);
        }
        CreateResponseDisposition::CompletedWithoutDocument => {
            signals.set_pending_document_create.set(None);
        }
        CreateResponseDisposition::Rejected(error) => {
            signals.set_pending_document_create.set(None);
            show_temporary_sync_banner(
                signals.sync_banner,
                signals.set_sync_banner,
                crate::i18n::server_error::message(locale, error.code).to_string(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ConnectionStatus;
    use crate::hooks::use_core::state::init_signals;
    use crate::runtime::document::create::PendingDocumentCreate;
    use deve_core::models::{DocId, RepoId};
    use deve_core::protocol::{DocumentCreateProjectionOutcome, DocumentCreateResponseContext};
    use leptos::prelude::{Owner, signal};

    #[test]
    fn document_create_response_settles_preobserved_exact_projection() {
        let owner = Owner::new();
        owner.set();
        let signals = init_signals(signal(ConnectionStatus::Connected).0);
        let repo_id = RepoId::new_v4();
        let pending = PendingDocumentCreate::new(repo_id, 7, "notes/a.md".into(), true);
        let request = pending.request();
        let doc_id = DocId(request.proposed_node_id.0);
        signals.set_pending_document_create.set(Some(pending));
        signals.set_docs.set(vec![(doc_id, "notes/a.md".into())]);

        handle(
            DocumentCreateResponse::Created {
                context: DocumentCreateResponseContext::from(&request),
                node_id: request.proposed_node_id,
                doc_id: Some(doc_id),
                path: request.path,
                projection_outcome: DocumentCreateProjectionOutcome::Written,
            },
            Locale::En,
            signals,
        );

        assert_eq!(signals.current_doc.get_untracked(), Some(doc_id));
        assert!(signals.pending_document_create.get_untracked().is_none());
    }
}
