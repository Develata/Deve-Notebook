//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_scope_nonce;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::{
    WriteGateAction, WriteGateReason, cannot_send, reason_from_block,
};
use crate::i18n::Locale;
use deve_core::protocol::ClientMessage;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::{Callback, GetUntracked, RwSignal, WriteSignal};

use super::SourceControlScopeSignals;

fn send_scoped(
    scope: SourceControlScopeSignals,
    ws: &WsService,
    build: impl FnOnce(u64) -> ClientMessage,
) {
    let Some(scope_nonce) = source_control_scope_nonce(scope) else {
        return;
    };
    ws.send(build(scope_nonce));
}

fn write_block_reason(ws: &WsService, gate: RepoWriteSignals) -> Option<WriteGateReason> {
    repo_write_block_untracked(ws, gate).map(reason_from_block)
}

pub(super) fn guarded_entry_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
    action: WriteGateAction,
    build: impl Fn(ChangeEntry, u64) -> ClientMessage + Clone + Send + Sync + 'static,
) -> Callback<ChangeEntry> {
    let ws = ws.clone();
    Callback::new(move |entry: ChangeEntry| {
        if let Some(reason) = write_block_reason(&ws, gate) {
            show_source_control_write_block(set_sync_banner, locale, action, reason);
            return;
        }
        let build = build.clone();
        send_scoped(scope, &ws, move |scope_nonce| build(entry, scope_nonce));
    })
}

pub(super) fn guarded_entries_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
    action: WriteGateAction,
    build: impl Fn(Vec<ChangeEntry>, u64) -> ClientMessage + Clone + Send + Sync + 'static,
) -> Callback<Vec<ChangeEntry>> {
    let ws = ws.clone();
    Callback::new(move |entries: Vec<ChangeEntry>| {
        if let Some(reason) = write_block_reason(&ws, gate) {
            show_source_control_write_block(set_sync_banner, locale, action, reason);
            return;
        }
        let build = build.clone();
        send_scoped(scope, &ws, move |scope_nonce| build(entries, scope_nonce));
    })
}

fn show_source_control_write_block(
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
    action: WriteGateAction,
    reason: WriteGateReason,
) {
    let message = cannot_send(locale.get_untracked(), action, reason);
    warn_sync_banner(set_sync_banner, message);
}

#[cfg(test)]
mod tests {
    use super::show_source_control_write_block;
    use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason};
    use crate::i18n::Locale;
    use leptos::prelude::{GetUntracked, signal};

    #[test]
    fn source_control_write_block_banner_includes_action_and_reason() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);
        let locale = leptos::prelude::RwSignal::new(Locale::Zh);

        show_source_control_write_block(
            set_sync_banner,
            locale,
            WriteGateAction::StageFile,
            WriteGateReason::ReadOnly,
        );

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("无法发送 暂存文件请求：只读模式")
        );
    }
}
