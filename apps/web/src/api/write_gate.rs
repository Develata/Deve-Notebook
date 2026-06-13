//! plan_ref:
//!   - 08_auth#unauthorized-disconnected-ui
//!   - 09_web_thin_client_ledger#write-readiness
//!

use leptos::prelude::*;

use super::status::ConnectionStatus;

#[derive(Clone, Copy)]
pub(in crate::api) struct WriterReadyResetSignals {
    set_writer_ready_repo_id: WriteSignal<Option<String>>,
    set_writer_ready_scope_nonce: WriteSignal<Option<u64>>,
    set_writer_client_id: WriteSignal<Option<u64>>,
}

impl WriterReadyResetSignals {
    pub(in crate::api) fn new(
        set_writer_ready_repo_id: WriteSignal<Option<String>>,
        set_writer_ready_scope_nonce: WriteSignal<Option<u64>>,
        set_writer_client_id: WriteSignal<Option<u64>>,
    ) -> Self {
        Self {
            set_writer_ready_repo_id,
            set_writer_ready_scope_nonce,
            set_writer_client_id,
        }
    }

    pub(in crate::api) fn clear(self) -> bool {
        self.set_writer_ready_repo_id.try_set(None).is_none()
            && self.set_writer_ready_scope_nonce.try_set(None).is_none()
            && self.set_writer_client_id.try_set(None).is_none()
    }
}

pub(in crate::api) fn status_revokes_writer_ready(status: ConnectionStatus) -> bool {
    !matches!(status, ConnectionStatus::Connected)
}

pub(in crate::api) fn set_status_and_revoke_writer_ready(
    set_status: WriteSignal<ConnectionStatus>,
    writer_ready_reset: WriterReadyResetSignals,
    status: ConnectionStatus,
) -> bool {
    if set_status.try_set(status).is_some() {
        return false;
    }
    !status_revokes_writer_ready(status) || writer_ready_reset.clear()
}
