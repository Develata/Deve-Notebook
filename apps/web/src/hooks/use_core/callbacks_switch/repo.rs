//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::{RepoRemoveRequest, RepoRenameRequest};
use deve_core::models::RepoId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::super::navigation::{NavigationTarget, guard_navigation};
use super::super::switch_nonce::next_switch_nonce_after;
use super::super::types::SwitchScopeSignals;
use super::{can_start_scope_switch, prepare_scope_switch, show_switch_block};

pub(super) fn build_switch_repo_callback(
    ws: WsService,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<String> {
    Callback::new(move |name: String| {
        if !can_start_scope_switch(signals) {
            show_switch_block(set_sync_banner, "switch repo", "scope switching");
            return;
        }
        if signals.current_repo.get_untracked().as_deref() == Some(name.as_str()) {
            return;
        }

        let target_repo = name.clone();
        let ws_repo_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let Some(switch_nonce) =
                next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
            else {
                show_switch_block(set_sync_banner, "switch repo", "scope nonce exhausted");
                return;
            };
            prepare_scope_switch(&ws_repo_action, signals);
            signals
                .set_pending_repo_switch
                .set(Some(target_repo.clone()));
            signals
                .set_pending_repo_switch_nonce
                .set(Some(switch_nonce));
            ws_repo_action.send(ClientMessage::SwitchRepo {
                name: target_repo.clone(),
                switch_nonce: Some(switch_nonce),
            });
        });
        let _ = guard_navigation(
            signals.current_doc.get_untracked(),
            signals.current_repo_id.get_untracked().as_deref(),
            signals.current_scope_nonce.get_untracked(),
            &signals.pending_local_edits.get_untracked(),
            signals.set_pending_navigation,
            NavigationTarget::Repo,
            action,
        );
    })
}

pub(super) fn build_create_repo_callback(
    ws: WsService,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<String> {
    Callback::new(move |name: String| {
        let target_repo = name.trim().to_string();
        if target_repo.is_empty() {
            show_switch_block(set_sync_banner, "create repo", "empty repository name");
            return;
        }
        if signals.active_branch.get_untracked().is_some() {
            show_switch_block(set_sync_banner, "create repo", "remote branch view");
            return;
        }
        if !can_start_scope_switch(signals) {
            show_switch_block(set_sync_banner, "create repo", "scope switching");
            return;
        }

        let ws_repo_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let Some(switch_nonce) =
                next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
            else {
                show_switch_block(set_sync_banner, "create repo", "scope nonce exhausted");
                return;
            };
            prepare_scope_switch(&ws_repo_action, signals);
            signals
                .set_pending_repo_switch
                .set(Some(target_repo.clone()));
            signals
                .set_pending_repo_switch_nonce
                .set(Some(switch_nonce));
            ws_repo_action.send(ClientMessage::CreateRepo {
                name: target_repo.clone(),
                switch_nonce: Some(switch_nonce),
            });
        });
        let _ = guard_navigation(
            signals.current_doc.get_untracked(),
            signals.current_repo_id.get_untracked().as_deref(),
            signals.current_scope_nonce.get_untracked(),
            &signals.pending_local_edits.get_untracked(),
            signals.set_pending_navigation,
            NavigationTarget::Repo,
            action,
        );
    })
}

pub(super) fn build_rename_repo_callback(
    ws: WsService,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<RepoRenameRequest> {
    Callback::new(move |request: RepoRenameRequest| {
        let target_repo = request.new_name.trim().to_string();
        if target_repo.is_empty() {
            show_switch_block(set_sync_banner, "rename repo", "empty repository name");
            return;
        }
        if signals.active_branch.get_untracked().is_some() {
            show_switch_block(set_sync_banner, "rename repo", "remote branch view");
            return;
        }
        if !can_start_scope_switch(signals) {
            show_switch_block(set_sync_banner, "rename repo", "scope switching");
            return;
        }

        let targets_current =
            repo_management_targets_current(signals, request.repo_id, &request.current_name);
        let ws_repo_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let Some(switch_nonce) =
                next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
            else {
                show_switch_block(set_sync_banner, "rename repo", "scope nonce exhausted");
                return;
            };
            if targets_current {
                prepare_scope_switch(&ws_repo_action, signals);
                signals
                    .set_pending_repo_switch
                    .set(Some(target_repo.clone()));
                signals
                    .set_pending_repo_switch_nonce
                    .set(Some(switch_nonce));
            }
            ws_repo_action.send(ClientMessage::RenameRepo {
                repo_id: request.repo_id,
                name: target_repo.clone(),
                switch_nonce: Some(switch_nonce),
            });
        });

        if targets_current {
            let _ = guard_navigation(
                signals.current_doc.get_untracked(),
                signals.current_repo_id.get_untracked().as_deref(),
                signals.current_scope_nonce.get_untracked(),
                &signals.pending_local_edits.get_untracked(),
                signals.set_pending_navigation,
                NavigationTarget::Repo,
                action,
            );
        } else {
            action.run(());
        }
    })
}

pub(super) fn build_remove_repo_callback(
    ws: WsService,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<RepoRemoveRequest> {
    Callback::new(move |request: RepoRemoveRequest| {
        if signals.active_branch.get_untracked().is_some() {
            show_switch_block(set_sync_banner, "remove repo", "remote branch view");
            return;
        }
        if !can_start_scope_switch(signals) {
            show_switch_block(set_sync_banner, "remove repo", "scope switching");
            return;
        }

        let targets_current =
            repo_management_targets_current(signals, request.repo_id, &request.current_name);
        if targets_current && request.fallback_name.as_deref().is_none() {
            show_switch_block(set_sync_banner, "remove repo", "no fallback repository");
            return;
        }

        let ws_repo_action = ws.clone();
        let action = Callback::new(move |_: ()| {
            let Some(switch_nonce) =
                next_switch_nonce_after(signals.current_scope_nonce.get_untracked())
            else {
                show_switch_block(set_sync_banner, "remove repo", "scope nonce exhausted");
                return;
            };
            if targets_current {
                prepare_scope_switch(&ws_repo_action, signals);
                signals
                    .set_pending_repo_switch
                    .set(request.fallback_name.clone());
                signals
                    .set_pending_repo_switch_nonce
                    .set(Some(switch_nonce));
            }
            ws_repo_action.send(ClientMessage::RemoveRepo {
                repo_id: request.repo_id,
                switch_nonce: Some(switch_nonce),
            });
        });

        if targets_current {
            let _ = guard_navigation(
                signals.current_doc.get_untracked(),
                signals.current_repo_id.get_untracked().as_deref(),
                signals.current_scope_nonce.get_untracked(),
                &signals.pending_local_edits.get_untracked(),
                signals.set_pending_navigation,
                NavigationTarget::Repo,
                action,
            );
        } else {
            action.run(());
        }
    })
}

fn repo_management_targets_current(
    signals: SwitchScopeSignals,
    repo_id: RepoId,
    current_name: &str,
) -> bool {
    let repo_id = repo_id.to_string();
    signals.current_repo_id.get_untracked().as_deref() == Some(repo_id.as_str())
        || signals.current_repo.get_untracked().as_deref() == Some(current_name)
}
