use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::i18n::{Locale, t};
use deve_core::protocol::{ServerError, ServerErrorCode};
use leptos::prelude::{GetUntracked, ReadSignal, Set, WriteSignal};

#[derive(Clone, Copy)]
pub struct ProtocolControlSignals {
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub set_pending_branch_switch: WriteSignal<Option<PendingBranchTarget>>,
    pub set_pending_branch_switch_nonce: WriteSignal<Option<u64>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub set_pending_repo_switch: WriteSignal<Option<String>>,
    pub set_pending_repo_switch_nonce: WriteSignal<Option<u64>>,
}

pub fn handle_protocol_error(
    ws: &WsService,
    locale: Locale,
    error: &ServerError,
    signals: ProtocolControlSignals,
) {
    clear_failed_scope_switch(error.code, signals);
    if is_auth_error(error.code) {
        ws.mark_unauthorized();
    }
    let message = t::server_error::message(locale, error.code);
    match error.detail.as_deref() {
        Some(detail) => leptos::logging::warn!("协议错误 {}: {}", message, detail),
        None => leptos::logging::warn!("协议错误 {}", message),
    }
    if let Some(window) = web_sys::window() {
        let _ = window.alert_with_message(message);
    }
}

fn clear_failed_scope_switch(code: ServerErrorCode, signals: ProtocolControlSignals) {
    if !is_scope_switch_error(code) {
        return;
    }
    if signals.pending_branch_switch.get_untracked().is_some() {
        signals.set_pending_branch_switch.set(None);
        signals.set_pending_branch_switch_nonce.set(None);
    }
    if signals.pending_repo_switch.get_untracked().is_some() {
        signals.set_pending_repo_switch.set(None);
        signals.set_pending_repo_switch_nonce.set(None);
    }
}

fn is_auth_error(code: ServerErrorCode) -> bool {
    matches!(
        code,
        ServerErrorCode::AuthTokenExpired | ServerErrorCode::AuthTokenMissing
    )
}

fn is_scope_switch_error(code: ServerErrorCode) -> bool {
    matches!(
        code,
        ServerErrorCode::ScRepoNotSelected
            | ServerErrorCode::ScRepoContextInvalid
            | ServerErrorCode::SyncRepoUnbound
    )
}

#[cfg(test)]
mod tests {
    use super::{ProtocolControlSignals, clear_failed_scope_switch};
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::protocol::ServerErrorCode;
    use leptos::prelude::*;

    #[test]
    fn switch_errors_clear_pending_scope_switches() {
        for code in [
            ServerErrorCode::ScRepoContextInvalid,
            ServerErrorCode::ScRepoNotSelected,
            ServerErrorCode::SyncRepoUnbound,
        ] {
            let runtime = leptos::reactive::owner::Owner::new();
            runtime.set();
            let (pending_branch_switch, set_pending_branch_switch) =
                signal(Some(PendingBranchTarget::Local));
            let (_, set_pending_branch_switch_nonce) = signal(Some(7u64));
            let (pending_repo_switch, set_pending_repo_switch) = signal(Some("wiki".to_string()));
            let (_, set_pending_repo_switch_nonce) = signal(Some(7u64));

            clear_failed_scope_switch(
                code,
                ProtocolControlSignals {
                    pending_branch_switch,
                    set_pending_branch_switch,
                    set_pending_branch_switch_nonce,
                    pending_repo_switch,
                    set_pending_repo_switch,
                    set_pending_repo_switch_nonce,
                },
            );

            assert_eq!(pending_branch_switch.get_untracked(), None);
            assert_eq!(pending_repo_switch.get_untracked(), None);
        }
    }

    #[test]
    fn non_switch_errors_keep_pending_scope_switches() {
        for code in [
            ServerErrorCode::AuthTokenExpired,
            ServerErrorCode::RequestFailed,
            ServerErrorCode::StoragePersistFailed,
            ServerErrorCode::StorageDbLocked,
        ] {
            let runtime = leptos::reactive::owner::Owner::new();
            runtime.set();
            let (pending_branch_switch, set_pending_branch_switch) =
                signal(Some(PendingBranchTarget::Local));
            let (_, set_pending_branch_switch_nonce) = signal(Some(7u64));
            let (pending_repo_switch, set_pending_repo_switch) = signal(Some("wiki".to_string()));
            let (_, set_pending_repo_switch_nonce) = signal(Some(7u64));

            clear_failed_scope_switch(
                code,
                ProtocolControlSignals {
                    pending_branch_switch,
                    set_pending_branch_switch,
                    set_pending_branch_switch_nonce,
                    pending_repo_switch,
                    set_pending_repo_switch,
                    set_pending_repo_switch_nonce,
                },
            );

            assert_eq!(
                pending_branch_switch.get_untracked(),
                Some(PendingBranchTarget::Local)
            );
            assert_eq!(
                pending_repo_switch.get_untracked(),
                Some("wiki".to_string())
            );
        }
    }
}
