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
    ExternalChangesMutationError, ExternalChangesTargetOp, apply_external_changes_to_ledger,
    fetch_external_changes, mutate_external_change_target,
};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;
use leptos::task::spawn_local;

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
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
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
    Callback::new(move |()| {
        let repo_id = scope.current_repo_id.get_untracked();
        let scope_nonce = scope.current_scope_nonce.get_untracked();
        let request_repo_id = repo_id.clone();
        spawn_local(async move {
            match fetch_external_changes(repo_id, scope_nonce).await {
                Ok(snapshot) => {
                    if !scope_is_current(scope, &request_repo_id, scope_nonce) {
                        return;
                    }
                    set_staged_changes.set(snapshot.staged);
                    set_unstaged_changes.set(snapshot.unstaged);
                }
                Err(error) => {
                    if scope_is_current(scope, &request_repo_id, scope_nonce) {
                        on_error.run(error);
                    }
                }
            }
        });
    })
}

pub fn create_external_changes_mutation_callbacks(
    scope: ExternalChangesHttpScope,
    on_refresh: Callback<()>,
    on_apply_confirmed_changes: Callback<Vec<ChangeEntry>>,
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
        on_apply_to_ledger: apply_callback(scope, on_refresh, on_apply_confirmed_changes, on_error),
    }
}

fn target_callback(
    op: ExternalChangesTargetOp,
    scope: ExternalChangesHttpScope,
    on_refresh: Callback<()>,
    on_error: Callback<ExternalChangesMutationError>,
) -> Callback<ChangeEntry> {
    Callback::new(move |entry: ChangeEntry| {
        let repo_id = scope.current_repo_id.get_untracked();
        let scope_nonce = scope.current_scope_nonce.get_untracked();
        let request_repo_id = repo_id.clone();
        spawn_local(async move {
            match mutate_external_change_target(op, repo_id, scope_nonce, entry).await {
                Ok(()) => {
                    if scope_is_current(scope, &request_repo_id, scope_nonce) {
                        on_refresh.run(());
                    }
                }
                Err(error) => {
                    if scope_is_current(scope, &request_repo_id, scope_nonce) {
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
        let repo_id = scope.current_repo_id.get_untracked();
        let scope_nonce = scope.current_scope_nonce.get_untracked();
        let request_repo_id = repo_id.clone();
        spawn_local(async move {
            for entry in entries {
                if !scope_is_current(scope, &request_repo_id, scope_nonce) {
                    return;
                }
                if let Err(error) =
                    mutate_external_change_target(op, repo_id.clone(), scope_nonce, entry).await
                {
                    if scope_is_current(scope, &request_repo_id, scope_nonce) {
                        on_error.run(error);
                    }
                    return;
                }
            }
            if scope_is_current(scope, &request_repo_id, scope_nonce) {
                on_refresh.run(());
            }
        });
    })
}

fn apply_callback(
    scope: ExternalChangesHttpScope,
    on_refresh: Callback<()>,
    on_apply_confirmed_changes: Callback<Vec<ChangeEntry>>,
    on_error: Callback<ExternalChangesMutationError>,
) -> Callback<()> {
    Callback::new(move |()| {
        let repo_id = scope.current_repo_id.get_untracked();
        let scope_nonce = scope.current_scope_nonce.get_untracked();
        let request_repo_id = repo_id.clone();
        spawn_local(async move {
            match apply_external_changes_to_ledger(repo_id, scope_nonce).await {
                Ok(confirmed_changes) => {
                    if !scope_is_current(scope, &request_repo_id, scope_nonce) {
                        return;
                    }
                    on_apply_confirmed_changes.run(confirmed_changes);
                    on_refresh.run(());
                }
                Err(error) => {
                    if scope_is_current(scope, &request_repo_id, scope_nonce) {
                        on_error.run(error);
                    }
                }
            }
        });
    })
}

fn scope_is_current(
    scope: ExternalChangesHttpScope,
    request_repo_id: &Option<String>,
    request_scope_nonce: u64,
) -> bool {
    let Some(current_repo_id) = scope.current_repo_id.try_get_untracked() else {
        return false;
    };
    let Some(current_scope_nonce) = scope.current_scope_nonce.try_get_untracked() else {
        return false;
    };
    scope_matches(
        request_repo_id,
        request_scope_nonce,
        &current_repo_id,
        current_scope_nonce,
    )
}

fn scope_matches(
    request_repo_id: &Option<String>,
    request_scope_nonce: u64,
    current_repo_id: &Option<String>,
    current_scope_nonce: u64,
) -> bool {
    request_repo_id == current_repo_id && request_scope_nonce == current_scope_nonce
}

#[cfg(test)]
mod tests {
    use super::scope_matches;

    #[test]
    fn external_changes_scope_guard_rejects_stale_responses() {
        assert!(scope_matches(
            &Some("repo-1".into()),
            7,
            &Some("repo-1".into()),
            7
        ));
        assert!(!scope_matches(
            &Some("repo-1".into()),
            7,
            &Some("repo-2".into()),
            7
        ));
        assert!(!scope_matches(
            &Some("repo-1".into()),
            7,
            &Some("repo-1".into()),
            8
        ));
    }
}
