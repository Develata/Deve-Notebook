//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_scope_nonce;
use crate::hooks::use_core::callbacks_sc_target::to_target;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::cannot_send;
use deve_core::protocol::ClientMessage;
use deve_core::source_control::{ChangeEntry, ConflictResolution};
use leptos::prelude::{Callback, Set, WriteSignal};

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

fn write_block_label(ws: &WsService, gate: RepoWriteSignals) -> Option<&'static str> {
    repo_write_block_untracked(ws, gate).map(|block| block.label())
}

fn show_write_block(
    set_sync_banner: WriteSignal<Option<String>>,
    action: &'static str,
    label: &str,
) {
    let message = cannot_send(action, label);
    warn_sync_banner(set_sync_banner, message);
}

fn show_commit_and_push_cli_only(set_notice: WriteSignal<Option<SourceControlNotice>>) {
    set_notice.set(Some(SourceControlNotice::git_push_cli_only()));
}

pub(super) fn create_commit_write_callbacks(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
    set_notice: WriteSignal<Option<SourceControlNotice>>,
    set_sync_banner: WriteSignal<Option<String>>,
) -> (
    Callback<String>,
    Callback<(ChangeEntry, ConflictResolution)>,
    Callback<String>,
) {
    let ws_commit = ws.clone();
    let on_commit = Callback::new(move |message: String| {
        if let Some(label) = write_block_label(&ws_commit, gate) {
            show_write_block(set_sync_banner, "Commit", label);
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
        if let Some(label) = write_block_label(&ws_conflict, gate) {
            show_write_block(set_sync_banner, "ResolveConflict", label);
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
        if let Some(label) = write_block_label(&ws_commit_and_push, gate) {
            show_write_block(set_sync_banner, "CommitAndPush", label);
            return;
        }
        show_commit_and_push_cli_only(set_notice);
    });
    (on_commit, on_resolve_conflict, on_commit_and_push)
}

#[cfg(test)]
mod tests {
    use super::{show_commit_and_push_cli_only, show_write_block};
    use crate::hooks::use_core::source_control_notice::is_git_push_cli_notice;
    use leptos::prelude::{GetUntracked, signal};

    #[test]
    fn commit_write_block_banner_includes_action_and_reason() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);

        show_write_block(set_sync_banner, "Commit", "repo handshaking");

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("Cannot send Commit: repo handshaking")
        );
    }

    #[test]
    fn resolve_conflict_block_banner_includes_action_and_reason() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);

        show_write_block(set_sync_banner, "ResolveConflict", "scope switching");

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("Cannot send ResolveConflict: scope switching")
        );
    }

    #[test]
    fn commit_and_push_callback_uses_git_push_cli_only_notice() {
        let (notice, set_notice) = signal(None);

        show_commit_and_push_cli_only(set_notice);

        assert_eq!(
            notice
                .get_untracked()
                .as_ref()
                .is_some_and(is_git_push_cli_notice),
            true
        );
    }
}
