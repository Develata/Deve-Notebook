//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_scope_nonce;
use crate::hooks::use_core::callbacks_sc_target::to_target;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::{
    WriteGateAction, WriteGateReason, cannot_send, reason_from_block,
};
use crate::i18n::{Locale, t};
use deve_core::protocol::ClientMessage;
use deve_core::source_control::{ChangeEntry, ConflictResolution};
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

fn show_write_block(
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
    action: WriteGateAction,
    reason: WriteGateReason,
) {
    let message = cannot_send(locale.get_untracked(), action, reason);
    warn_sync_banner(set_sync_banner, message);
}

fn show_commit_and_push_cli_only(
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
) {
    warn_sync_banner(
        set_sync_banner,
        t::source_control::commit_and_push_cli_only_banner(locale.get_untracked()).to_string(),
    );
}

pub(super) fn create_commit_write_callbacks(
    ws: &WsService,
    locale: RwSignal<Locale>,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> (
    Callback<String>,
    Callback<(ChangeEntry, ConflictResolution)>,
    Callback<String>,
) {
    let ws_commit = ws.clone();
    let on_commit = Callback::new(move |message: String| {
        if let Some(reason) = write_block_reason(&ws_commit, gate) {
            show_write_block(set_sync_banner, locale, WriteGateAction::Commit, reason);
            return;
        }
        send_scoped(scope, &ws_commit, move |scope_nonce| {
            ClientMessage::Commit {
                message,
                scope_nonce: Some(scope_nonce),
            }
        });
    });
    let ws_conflict = ws.clone();
    let on_resolve_conflict = Callback::new(move |(entry, resolution)| {
        if let Some(reason) = write_block_reason(&ws_conflict, gate) {
            show_write_block(
                set_sync_banner,
                locale,
                WriteGateAction::ResolveConflict,
                reason,
            );
            return;
        }
        send_scoped(scope, &ws_conflict, move |scope_nonce| {
            ClientMessage::ResolveConflict {
                target: to_target(&entry),
                resolution,
                scope_nonce: Some(scope_nonce),
            }
        });
    });
    let ws_commit_and_push = ws.clone();
    let on_commit_and_push = Callback::new(move |_message: String| {
        if let Some(reason) = write_block_reason(&ws_commit_and_push, gate) {
            show_write_block(
                set_sync_banner,
                locale,
                WriteGateAction::CommitAndPush,
                reason,
            );
            return;
        }
        show_commit_and_push_cli_only(set_sync_banner, locale);
    });
    (on_commit, on_resolve_conflict, on_commit_and_push)
}

#[cfg(test)]
mod tests {
    use super::{show_commit_and_push_cli_only, show_write_block};
    use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason};
    use crate::i18n::Locale;
    use leptos::prelude::{GetUntracked, signal};

    #[test]
    fn commit_write_block_banner_includes_action_and_reason() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);
        let locale = leptos::prelude::RwSignal::new(Locale::Zh);

        show_write_block(
            set_sync_banner,
            locale,
            WriteGateAction::Commit,
            WriteGateReason::HandshakingRepo,
        );

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("无法发送 提交请求：正在协商仓库写入权限")
        );
    }

    #[test]
    fn resolve_conflict_block_banner_includes_action_and_reason() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);
        let locale = leptos::prelude::RwSignal::new(Locale::Zh);

        show_write_block(
            set_sync_banner,
            locale,
            WriteGateAction::ResolveConflict,
            WriteGateReason::ScopeSwitching,
        );

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("无法发送 解决冲突请求：正在切换作用域")
        );
    }

    #[test]
    fn commit_and_push_callback_uses_cli_only_banner() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);
        let locale = leptos::prelude::RwSignal::new(Locale::Zh);

        show_commit_and_push_cli_only(set_sync_banner, locale);

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("Commit & Push 只能通过 CLI 完成；请先创建提交，再运行 `deve_cli ngit push`。")
        );
    }
}
