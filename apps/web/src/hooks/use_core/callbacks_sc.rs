use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::callbacks_sc_scope::source_control_scope_nonce;
use crate::hooks::use_core::callbacks_sc_target::{to_target, to_targets};
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use deve_core::protocol::ClientMessage;
use deve_core::source_control::{ChangeEntry, ConflictResolution};
use leptos::prelude::*;

pub struct SourceControlCallbacks {
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<ChangeEntry>,
    pub on_stage_files: Callback<Vec<ChangeEntry>>,
    pub on_unstage_file: Callback<ChangeEntry>,
    pub on_unstage_files: Callback<Vec<ChangeEntry>>,
    pub on_discard_file: Callback<ChangeEntry>,
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub on_get_doc_diff: Callback<ChangeEntry>,
    pub on_resolve_conflict: Callback<(ChangeEntry, ConflictResolution)>,
    pub on_get_commit_diff: Callback<(Option<String>, String)>,
    pub on_commit_and_push: Callback<String>,
}

#[derive(Clone, Copy)]
pub struct SourceControlScopeSignals {
    pub current_repo_id: ReadSignal<Option<String>>,
    pub active_branch: ReadSignal<Option<deve_core::models::PeerId>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
}

#[derive(Clone, Copy)]
pub struct SourceControlRequestSignals {
    pub set_changes_request_id: WriteSignal<Option<String>>,
    pub set_commit_history_request_id: WriteSignal<Option<String>>,
    pub set_doc_diff_request_id: WriteSignal<Option<String>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
}

pub fn create_source_control_callbacks(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    write_gate: RepoWriteSignals,
    request: SourceControlRequestSignals,
) -> SourceControlCallbacks {
    let ws_changes = ws.clone();
    let on_get_changes = Callback::new(move |_: ()| {
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        request.set_changes_request_id.set(Some(request_id.clone()));
        ws_changes.send(ClientMessage::GetChanges {
            request_id,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_stage = ws.clone();
    let on_stage_file = Callback::new(move |entry: ChangeEntry| {
        if let Some(block) = repo_write_block_untracked(&ws_stage, write_gate) {
            leptos::logging::warn!("忽略 StageFile: {}", block.label());
            return;
        }
        send_targeted(scope, &ws_stage, move |scope_nonce| {
            ClientMessage::StageFile {
                target: to_target(&entry),
                scope_nonce: Some(scope_nonce),
            }
        });
    });

    let ws_stage_many = ws.clone();
    let on_stage_files = Callback::new(move |entries: Vec<ChangeEntry>| {
        if let Some(block) = repo_write_block_untracked(&ws_stage_many, write_gate) {
            leptos::logging::warn!("忽略 StageFiles: {}", block.label());
            return;
        }
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let targets = to_targets(entries);
        if targets.is_empty() {
            return;
        }
        ws_stage_many.send(ClientMessage::StageFiles {
            targets,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_unstage = ws.clone();
    let on_unstage_file = Callback::new(move |entry: ChangeEntry| {
        if let Some(block) = repo_write_block_untracked(&ws_unstage, write_gate) {
            leptos::logging::warn!("忽略 UnstageFile: {}", block.label());
            return;
        }
        send_targeted(scope, &ws_unstage, move |scope_nonce| {
            ClientMessage::UnstageFile {
                target: to_target(&entry),
                scope_nonce: Some(scope_nonce),
            }
        });
    });

    let ws_unstage_many = ws.clone();
    let on_unstage_files = Callback::new(move |entries: Vec<ChangeEntry>| {
        if let Some(block) = repo_write_block_untracked(&ws_unstage_many, write_gate) {
            leptos::logging::warn!("忽略 UnstageFiles: {}", block.label());
            return;
        }
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let targets = to_targets(entries);
        if targets.is_empty() {
            return;
        }
        ws_unstage_many.send(ClientMessage::UnstageFiles {
            targets,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_discard = ws.clone();
    let on_discard_file = Callback::new(move |entry: ChangeEntry| {
        if let Some(block) = repo_write_block_untracked(&ws_discard, write_gate) {
            leptos::logging::warn!("忽略 DiscardFile: {}", block.label());
            return;
        }
        send_targeted(scope, &ws_discard, move |scope_nonce| {
            ClientMessage::DiscardFile {
                target: to_target(&entry),
                scope_nonce: Some(scope_nonce),
            }
        });
    });

    let ws_commit = ws.clone();
    let on_commit = Callback::new(move |message: String| {
        if let Some(block) = repo_write_block_untracked(&ws_commit, write_gate) {
            leptos::logging::warn!("忽略 Commit: {}", block.label());
            return;
        }
        send_simple(scope, &ws_commit, move |scope_nonce| {
            ClientMessage::Commit {
                message,
                scope_nonce: Some(scope_nonce),
            }
        });
    });

    let ws_history = ws.clone();
    let on_get_history = Callback::new(move |limit: u32| {
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        request
            .set_commit_history_request_id
            .set(Some(request_id.clone()));
        ws_history.send(ClientMessage::GetCommitHistory {
            request_id,
            limit,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_doc_diff = ws.clone();
    let on_get_doc_diff = Callback::new(move |entry: ChangeEntry| {
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        request
            .set_doc_diff_request_id
            .set(Some(request_id.clone()));
        ws_doc_diff.send(ClientMessage::GetDocDiff {
            request_id,
            target: to_target(&entry),
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_conflict = ws.clone();
    let on_resolve_conflict = Callback::new(
        move |(entry, resolution): (ChangeEntry, ConflictResolution)| {
            if let Some(block) = repo_write_block_untracked(&ws_conflict, write_gate) {
                leptos::logging::warn!("忽略 ResolveConflict: {}", block.label());
                return;
            }
            send_simple(scope, &ws_conflict, move |scope_nonce| {
                ClientMessage::ResolveConflict {
                    target: to_target(&entry),
                    resolution,
                    scope_nonce: Some(scope_nonce),
                }
            });
        },
    );

    let ws_commit_diff = ws.clone();
    let on_get_commit_diff = Callback::new(move |(commit_a, commit_b)| {
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        request
            .set_commit_diff_request_id
            .set(Some(request_id.clone()));
        ws_commit_diff.send(ClientMessage::GetCommitDiff {
            request_id,
            commit_a,
            commit_b,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_commit_and_push = ws.clone();
    let on_commit_and_push = Callback::new(move |message: String| {
        if let Some(block) = repo_write_block_untracked(&ws_commit_and_push, write_gate) {
            leptos::logging::warn!("忽略 CommitAndPush: {}", block.label());
            return;
        }
        send_simple(scope, &ws_commit_and_push, move |scope_nonce| {
            ClientMessage::CommitAndPush {
                message,
                scope_nonce: Some(scope_nonce),
            }
        });
    });

    SourceControlCallbacks {
        on_get_changes,
        on_stage_file,
        on_stage_files,
        on_unstage_file,
        on_unstage_files,
        on_discard_file,
        on_commit,
        on_get_history,
        on_get_doc_diff,
        on_resolve_conflict,
        on_get_commit_diff,
        on_commit_and_push,
    }
}

fn send_simple(
    scope: SourceControlScopeSignals,
    ws: &WsService,
    build: impl FnOnce(u64) -> ClientMessage,
) {
    let Some(scope_nonce) = source_control_scope_nonce(scope) else {
        return;
    };
    ws.send(build(scope_nonce));
}

fn send_targeted(
    scope: SourceControlScopeSignals,
    ws: &WsService,
    build: impl FnOnce(u64) -> ClientMessage,
) {
    send_simple(scope, ws, build);
}
