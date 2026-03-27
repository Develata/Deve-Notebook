use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_scope_nonce;
use crate::hooks::use_core::callbacks_sc_target::to_target;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use deve_core::protocol::ClientMessage;
use deve_core::source_control::{ChangeEntry, ConflictResolution};
use leptos::prelude::Callback;

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

pub(super) fn create_commit_write_callbacks(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    gate: RepoWriteSignals,
) -> (
    Callback<String>,
    Callback<(ChangeEntry, ConflictResolution)>,
    Callback<String>,
) {
    let ws_commit = ws.clone();
    let on_commit = Callback::new(move |message: String| {
        if let Some(label) = write_block_label(&ws_commit, gate) {
            leptos::logging::warn!("忽略 Commit: {}", label);
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
            leptos::logging::warn!("忽略 ResolveConflict: {}", label);
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
    let on_commit_and_push = Callback::new(move |message: String| {
        if let Some(label) = write_block_label(&ws_commit_and_push, gate) {
            leptos::logging::warn!("忽略 CommitAndPush: {}", label);
            return;
        }
        send_scoped(scope, &ws_commit_and_push, move |scope_nonce| {
            ClientMessage::CommitAndPush {
                message,
                scope_nonce: Some(scope_nonce),
            }
        });
    });
    (on_commit, on_resolve_conflict, on_commit_and_push)
}
