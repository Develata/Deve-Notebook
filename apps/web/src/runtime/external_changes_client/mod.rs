//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! External Changes runtime facade.
//!
//! This client exposes the browser intents for projection-file changes only.
//! Source Control commit/history/graph state remains outside this facade.

use crate::api::{
    ExternalChangesMutationError, ExternalChangesTargetOp, WsService, fetch_external_changes,
    mutate_external_change_target,
};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[allow(dead_code)]
#[derive(Clone)]
pub struct ExternalChangesClient {
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<ChangeEntry>,
    pub on_stage_files: Callback<Vec<ChangeEntry>>,
    pub on_unstage_file: Callback<ChangeEntry>,
    pub on_unstage_files: Callback<Vec<ChangeEntry>>,
    pub on_discard_file: Callback<ChangeEntry>,
    pub on_apply_to_ledger: Callback<()>,
    pub on_get_doc_diff: Callback<ChangeEntry>,
}

#[derive(Clone, Copy)]
pub struct ExternalChangesHttpScope {
    pub current_connection_epoch: ReadSignal<u64>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExternalChangesRequestScope {
    connection_epoch: u64,
    repo_id: Option<String>,
    scope_nonce: u64,
}

pub struct ExternalChangesMutationCallbacks {
    pub on_stage_file: Callback<ChangeEntry>,
    pub on_stage_files: Callback<Vec<ChangeEntry>>,
    pub on_unstage_file: Callback<ChangeEntry>,
    pub on_unstage_files: Callback<Vec<ChangeEntry>>,
    pub on_discard_file: Callback<ChangeEntry>,
    pub on_apply_to_ledger: Callback<()>,
}

pub fn create_external_changes_refresh_callback(
    scope: ExternalChangesHttpScope,
    set_staged_changes: WriteSignal<Vec<ChangeEntry>>,
    set_unstaged_changes: WriteSignal<Vec<ChangeEntry>>,
    on_error: Callback<ExternalChangesMutationError>,
) -> Callback<()> {
    let latest_request_generation = Arc::new(AtomicU64::new(0u64));
    Callback::new(move |()| {
        let request_generation = latest_request_generation
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1)
            .max(1);
        let response_generation = latest_request_generation.clone();
        let request_scope = capture_scope(scope);
        let repo_id = request_scope.repo_id.clone();
        let scope_nonce = request_scope.scope_nonce;
        spawn_local(async move {
            match fetch_external_changes(repo_id, scope_nonce).await {
                Ok(snapshot) => {
                    if !request_is_current(
                        response_generation.load(Ordering::Relaxed),
                        request_generation,
                    ) || !scope_is_current(scope, &request_scope)
                    {
                        return;
                    }
                    set_staged_changes.set(snapshot.staged);
                    set_unstaged_changes.set(snapshot.unstaged);
                }
                Err(error) => {
                    if request_is_current(
                        response_generation.load(Ordering::Relaxed),
                        request_generation,
                    ) && scope_is_current(scope, &request_scope)
                    {
                        on_error.run(error);
                    }
                }
            }
        });
    })
}

pub fn create_external_changes_mutation_callbacks(
    scope: ExternalChangesHttpScope,
    ws: WsService,
    on_refresh: Callback<()>,
    on_error: Callback<ExternalChangesMutationError>,
) -> ExternalChangesMutationCallbacks {
    ExternalChangesMutationCallbacks {
        on_stage_file: target_callback(
            ExternalChangesTargetOp::Stage,
            scope,
            on_refresh.clone(),
            on_error.clone(),
        ),
        on_stage_files: targets_callback(
            ExternalChangesTargetOp::Stage,
            scope,
            on_refresh.clone(),
            on_error.clone(),
        ),
        on_unstage_file: target_callback(
            ExternalChangesTargetOp::Unstage,
            scope,
            on_refresh.clone(),
            on_error.clone(),
        ),
        on_unstage_files: targets_callback(
            ExternalChangesTargetOp::Unstage,
            scope,
            on_refresh.clone(),
            on_error.clone(),
        ),
        on_discard_file: target_callback(
            ExternalChangesTargetOp::Discard,
            scope,
            on_refresh.clone(),
            on_error.clone(),
        ),
        on_apply_to_ledger: apply_callback(scope, ws),
    }
}

fn target_callback(
    op: ExternalChangesTargetOp,
    scope: ExternalChangesHttpScope,
    on_refresh: Callback<()>,
    on_error: Callback<ExternalChangesMutationError>,
) -> Callback<ChangeEntry> {
    Callback::new(move |entry: ChangeEntry| {
        let request_scope = capture_scope(scope);
        let repo_id = request_scope.repo_id.clone();
        let scope_nonce = request_scope.scope_nonce;
        spawn_local(async move {
            match mutate_external_change_target(op, repo_id, scope_nonce, entry).await {
                Ok(()) => {
                    if scope_is_current(scope, &request_scope) {
                        on_refresh.run(());
                    }
                }
                Err(error) => {
                    if scope_is_current(scope, &request_scope) {
                        on_error.run(error);
                    }
                }
            }
        });
    })
}

fn targets_callback(
    op: ExternalChangesTargetOp,
    scope: ExternalChangesHttpScope,
    on_refresh: Callback<()>,
    on_error: Callback<ExternalChangesMutationError>,
) -> Callback<Vec<ChangeEntry>> {
    Callback::new(move |entries: Vec<ChangeEntry>| {
        if entries.is_empty() {
            return;
        }
        let request_scope = capture_scope(scope);
        let repo_id = request_scope.repo_id.clone();
        let scope_nonce = request_scope.scope_nonce;
        spawn_local(async move {
            for entry in entries {
                if !scope_is_current(scope, &request_scope) {
                    return;
                }
                if let Err(error) =
                    mutate_external_change_target(op, repo_id.clone(), scope_nonce, entry).await
                {
                    if scope_is_current(scope, &request_scope) {
                        on_error.run(error);
                    }
                    return;
                }
            }
            if scope_is_current(scope, &request_scope) {
                on_refresh.run(());
            }
        });
    })
}

fn apply_callback(scope: ExternalChangesHttpScope, ws: WsService) -> Callback<()> {
    Callback::new(move |()| {
        if scope.current_repo_id.get_untracked().is_none() {
            return;
        }
        let scope_nonce = scope.current_scope_nonce.get_untracked();
        ws.request_external_apply(scope_nonce);
    })
}

fn scope_is_current(
    scope: ExternalChangesHttpScope,
    request: &ExternalChangesRequestScope,
) -> bool {
    let Some(current_connection_epoch) = scope.current_connection_epoch.try_get_untracked() else {
        return false;
    };
    let Some(current_repo_id) = scope.current_repo_id.try_get_untracked() else {
        return false;
    };
    let Some(current_scope_nonce) = scope.current_scope_nonce.try_get_untracked() else {
        return false;
    };
    scope_matches(
        request,
        current_connection_epoch,
        &current_repo_id,
        current_scope_nonce,
    )
}

fn capture_scope(scope: ExternalChangesHttpScope) -> ExternalChangesRequestScope {
    ExternalChangesRequestScope {
        connection_epoch: scope.current_connection_epoch.get_untracked(),
        repo_id: scope.current_repo_id.get_untracked(),
        scope_nonce: scope.current_scope_nonce.get_untracked(),
    }
}

fn scope_matches(
    request: &ExternalChangesRequestScope,
    current_connection_epoch: u64,
    current_repo_id: &Option<String>,
    current_scope_nonce: u64,
) -> bool {
    request.connection_epoch == current_connection_epoch
        && &request.repo_id == current_repo_id
        && request.scope_nonce == current_scope_nonce
}

fn request_is_current(latest_generation: u64, response_generation: u64) -> bool {
    latest_generation == response_generation
}

#[cfg(test)]
mod tests {
    use super::{ExternalChangesRequestScope, request_is_current, scope_matches};

    #[test]
    fn external_changes_scope_guard_rejects_stale_responses() {
        let request = ExternalChangesRequestScope {
            connection_epoch: 3,
            repo_id: Some("repo-1".into()),
            scope_nonce: 7,
        };
        assert!(scope_matches(&request, 3, &Some("repo-1".into()), 7));
        assert!(!scope_matches(&request, 3, &Some("repo-2".into()), 7));
        assert!(!scope_matches(&request, 3, &Some("repo-1".into()), 8));
        assert!(!scope_matches(&request, 4, &Some("repo-1".into()), 7));
    }

    #[test]
    fn external_changes_request_generation_rejects_older_same_scope_response() {
        assert!(request_is_current(4, 4));
        assert!(!request_is_current(5, 4));
    }
}
