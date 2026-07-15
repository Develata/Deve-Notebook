//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#projection-recovery-contract
//!
//! Correlated Source Control mutation intents owned by the WS transport.

use super::WsService;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set};

impl WsService {
    pub(crate) fn request_external_apply(&self, scope_nonce: u64) -> String {
        let request_id = uuid::Uuid::new_v4().to_string();
        self.set_external_apply_request_id
            .set(Some(request_id.clone()));
        self.send(ClientMessage::ApplyExternalChanges {
            request_id: request_id.clone(),
            scope_nonce: Some(scope_nonce),
        });
        request_id
    }

    pub(crate) fn complete_external_apply(&self, request_id: &str) -> bool {
        if self.external_apply_request_id.get_untracked().as_deref() != Some(request_id) {
            return false;
        }
        self.set_external_apply_request_id.set(None);
        true
    }
}
