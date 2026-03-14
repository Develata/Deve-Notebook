use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::callbacks_sc_target::{
    resolve_target, resolve_target_any, resolve_targets,
};
use deve_core::protocol::ClientMessage;
use deve_core::source_control::ChangeEntry;
use deve_core::source_control::ConflictResolution;
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
    let ws1 = ws.clone();
    let on_get_changes = Callback::new(move |_: ()| {
        if !source_control_scope_ready(
            scope.current_repo_id.get_untracked(),
            scope.pending_branch_switch.get_untracked(),
            scope.pending_repo_switch.get_untracked(),
        ) {
            return;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        request.set_changes_request_id.set(Some(request_id.clone()));
        ws1.send(ClientMessage::GetChanges { request_id });
    });

    let ws2 = ws.clone();
    let on_stage_file = Callback::new(move |path: String| {
        if !source_control_scope_ready(
            scope.current_repo_id.get_untracked(),
            scope.pending_branch_switch.get_untracked(),
            scope.pending_repo_switch.get_untracked(),
        ) {
            return;
        }
        ws2.send(ClientMessage::StageFile {
            target: resolve_target(unstaged_changes, &path),
        });
    });

    let ws3 = ws.clone();
    let on_unstage_file = Callback::new(move |path: String| {
        if !source_control_scope_ready(
            scope.current_repo_id.get_untracked(),
            scope.pending_branch_switch.get_untracked(),
            scope.pending_repo_switch.get_untracked(),
        ) {
            return;
        }
        ws3.send(ClientMessage::UnstageFile {
            target: resolve_target(staged_changes, &path),
        });
    });

    let ws3b = ws.clone();
    let on_stage_files = Callback::new(move |paths: Vec<String>| {
        if !source_control_scope_ready(
            scope.current_repo_id.get_untracked(),
            scope.pending_branch_switch.get_untracked(),
            scope.pending_repo_switch.get_untracked(),
        ) {
            return;
        }
        let targets = resolve_targets(unstaged_changes, paths);
        if targets.is_empty() {
            return;
        }
        ws3b.send(ClientMessage::StageFiles { targets });
    });

    let ws3c = ws.clone();
    let on_unstage_files = Callback::new(move |paths: Vec<String>| {
        if !source_control_scope_ready(
            scope.current_repo_id.get_untracked(),
            scope.pending_branch_switch.get_untracked(),
            scope.pending_repo_switch.get_untracked(),
        ) {
            return;
        }
        let targets = resolve_targets(staged_changes, paths);
        if targets.is_empty() {
            return;
        }
        ws3c.send(ClientMessage::UnstageFiles { targets });
    });

    let ws4 = ws.clone();
    let on_commit = Callback::new(move |message: String| {
        if !source_control_scope_ready(
            scope.current_repo_id.get_untracked(),
            scope.pending_branch_switch.get_untracked(),
            scope.pending_repo_switch.get_untracked(),
        ) {
            return;
        }
        ws4.send(ClientMessage::Commit { message });
    });

    let ws5 = ws.clone();
    let on_get_history = Callback::new(move |limit: u32| {
        if !source_control_scope_ready(
            scope.current_repo_id.get_untracked(),
            scope.pending_branch_switch.get_untracked(),
            scope.pending_repo_switch.get_untracked(),
        ) {
            return;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        request
            .set_commit_history_request_id
            .set(Some(request_id.clone()));
        ws5.send(ClientMessage::GetCommitHistory { request_id, limit });
    });

    let ws6 = ws.clone();
    let on_get_doc_diff = Callback::new(move |path: String| {
        if !source_control_scope_ready(
            scope.current_repo_id.get_untracked(),
            scope.pending_branch_switch.get_untracked(),
            scope.pending_repo_switch.get_untracked(),
        ) {
            return;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        request
            .set_doc_diff_request_id
            .set(Some(request_id.clone()));
        ws6.send(ClientMessage::GetDocDiff {
            request_id,
            target: resolve_target_any(staged_changes, unstaged_changes, &path),
        });
    });

    let ws7 = ws.clone();
    let on_discard_file = Callback::new(move |path: String| {
        if !source_control_scope_ready(
            scope.current_repo_id.get_untracked(),
            scope.pending_branch_switch.get_untracked(),
            scope.pending_repo_switch.get_untracked(),
        ) {
            return;
        }
        ws7.send(ClientMessage::DiscardFile {
            target: resolve_target(unstaged_changes, &path),
        });
    });

    let ws8 = ws.clone();
    let on_resolve_conflict =
        Callback::new(move |(path, resolution): (String, ConflictResolution)| {
            if !source_control_scope_ready(
                scope.current_repo_id.get_untracked(),
                scope.pending_branch_switch.get_untracked(),
                scope.pending_repo_switch.get_untracked(),
            ) {
                return;
            }
            ws8.send(ClientMessage::ResolveConflict {
                target: resolve_target(unstaged_changes, &path),
                resolution,
            });
        });

    let ws9 = ws.clone();
    let on_get_commit_diff =
        Callback::new(move |(commit_a, commit_b): (Option<String>, String)| {
            if !source_control_scope_ready(
                scope.current_repo_id.get_untracked(),
                scope.pending_branch_switch.get_untracked(),
                scope.pending_repo_switch.get_untracked(),
            ) {
                return;
            }
            let request_id = uuid::Uuid::new_v4().to_string();
            request
                .set_commit_diff_request_id
                .set(Some(request_id.clone()));
            ws9.send(ClientMessage::GetCommitDiff {
                request_id,
                commit_a,
                commit_b,
            });
        });

    let ws10 = ws.clone();
    let on_commit_and_push = Callback::new(move |message: String| {
        if !source_control_scope_ready(
            scope.current_repo_id.get_untracked(),
            scope.pending_branch_switch.get_untracked(),
            scope.pending_repo_switch.get_untracked(),
        ) {
            return;
        }
        ws10.send(ClientMessage::CommitAndPush { message });
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

fn source_control_scope_ready(
    current_repo_id: Option<String>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
) -> bool {
    current_repo_id.is_some() && pending_branch_switch.is_none() && pending_repo_switch.is_none()
}

#[cfg(test)]
mod tests {
    use super::source_control_scope_ready;
    use crate::hooks::use_core::PendingBranchTarget;

    #[test]
    fn source_control_scope_requires_bound_repo_and_no_pending_switch() {
        assert!(!source_control_scope_ready(None, None, None));
        assert!(!source_control_scope_ready(
            Some("repo-a".into()),
            Some(PendingBranchTarget::Local),
            None,
        ));
        assert!(!source_control_scope_ready(
            Some("repo-a".into()),
            None,
            Some("repo-b".into()),
        ));
        assert!(source_control_scope_ready(
            Some("repo-a".into()),
            None,
            None,
        ));
    }
}
