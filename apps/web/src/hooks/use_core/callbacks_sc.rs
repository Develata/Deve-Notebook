use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::callbacks_sc_scope::source_control_scope_nonce;
use crate::hooks::use_core::callbacks_sc_target::{
    resolve_target, resolve_target_any, resolve_targets,
};
use deve_core::protocol::ClientMessage;
use deve_core::source_control::{ChangeEntry, ConflictResolution};
use leptos::prelude::*;

pub struct SourceControlCallbacks {
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<String>,
    pub on_stage_files: Callback<Vec<String>>,
    pub on_unstage_file: Callback<String>,
    pub on_unstage_files: Callback<Vec<String>>,
    pub on_discard_file: Callback<String>,
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub on_get_doc_diff: Callback<String>,
    pub on_resolve_conflict: Callback<(String, ConflictResolution)>,
    pub on_get_commit_diff: Callback<(Option<String>, String)>,
    pub on_commit_and_push: Callback<String>,
}

#[derive(Clone, Copy)]
pub struct SourceControlScopeSignals {
    pub current_repo_id: ReadSignal<Option<String>>,
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
    staged_changes: ReadSignal<Vec<ChangeEntry>>,
    unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    scope: SourceControlScopeSignals,
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
    let on_stage_file = Callback::new(move |path: String| {
        send_targeted(scope, &ws_stage, move |scope_nonce| {
            ClientMessage::StageFile {
                target: resolve_target(unstaged_changes, &path),
                scope_nonce: Some(scope_nonce),
            }
        });
    });

    let ws_stage_many = ws.clone();
    let on_stage_files = Callback::new(move |paths: Vec<String>| {
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let targets = resolve_targets(unstaged_changes, paths);
        if targets.is_empty() {
            return;
        }
        ws_stage_many.send(ClientMessage::StageFiles {
            targets,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_unstage = ws.clone();
    let on_unstage_file = Callback::new(move |path: String| {
        send_targeted(scope, &ws_unstage, move |scope_nonce| {
            ClientMessage::UnstageFile {
                target: resolve_target(staged_changes, &path),
                scope_nonce: Some(scope_nonce),
            }
        });
    });

    let ws_unstage_many = ws.clone();
    let on_unstage_files = Callback::new(move |paths: Vec<String>| {
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let targets = resolve_targets(staged_changes, paths);
        if targets.is_empty() {
            return;
        }
        ws_unstage_many.send(ClientMessage::UnstageFiles {
            targets,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_discard = ws.clone();
    let on_discard_file = Callback::new(move |path: String| {
        send_targeted(scope, &ws_discard, move |scope_nonce| {
            ClientMessage::DiscardFile {
                target: resolve_target(unstaged_changes, &path),
                scope_nonce: Some(scope_nonce),
            }
        });
    });

    let ws_commit = ws.clone();
    let on_commit = Callback::new(move |message: String| {
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
    let on_get_doc_diff = Callback::new(move |path: String| {
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        request
            .set_doc_diff_request_id
            .set(Some(request_id.clone()));
        ws_doc_diff.send(ClientMessage::GetDocDiff {
            request_id,
            target: resolve_target_any(staged_changes, unstaged_changes, &path),
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws_conflict = ws.clone();
    let on_resolve_conflict =
        Callback::new(move |(path, resolution): (String, ConflictResolution)| {
            send_simple(scope, &ws_conflict, move |scope_nonce| {
                ClientMessage::ResolveConflict {
                    target: resolve_target(unstaged_changes, &path),
                    resolution,
                    scope_nonce: Some(scope_nonce),
                }
            });
        });

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
