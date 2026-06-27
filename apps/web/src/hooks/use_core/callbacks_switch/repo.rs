//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::{RepoRemoveRequest, RepoRenameRequest, RepoSwitchRequest};
use deve_core::models::RepoId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::super::navigation::{NavigationTarget, guard_navigation};
use super::super::switch_nonce::next_switch_nonce_after;
use super::super::types::{PendingRepoSwitch, SwitchScopeSignals};
use super::{can_start_scope_switch, prepare_scope_switch, show_switch_block};

pub(super) fn build_switch_repo_callback(
    ws: WsService,
    signals: SwitchScopeSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> Callback<RepoSwitchRequest> {
    Callback::new(move |request: RepoSwitchRequest| {
        if !can_start_scope_switch(signals) {
            show_switch_block(set_sync_banner, "switch repo", "scope switching");
            return;
        }
        if repo_switch_request_targets_current(&request, signals) {
            return;
        }

        let selector_name = request.selector_name.clone();
        let expected_name = request.expected_name.clone();
        let repo_id = request.repo_id;
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
                .set(Some(PendingRepoSwitch::switch(
                    expected_name.clone(),
                    switch_nonce,
                )));
            if let Some(repo_id) = repo_id {
                ws_repo_action.send(ClientMessage::SwitchRepoExact {
                    name: selector_name.clone(),
                    repo_id,
                    switch_nonce: Some(switch_nonce),
                });
            } else {
                ws_repo_action.send(ClientMessage::SwitchRepo {
                    name: selector_name.clone(),
                    switch_nonce: Some(switch_nonce),
                });
            }
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

fn repo_switch_request_targets_current(
    request: &RepoSwitchRequest,
    signals: SwitchScopeSignals,
) -> bool {
    if let Some(repo_id) = request.repo_id {
        let repo_id = repo_id.to_string();
        return signals.current_repo_id.get_untracked().as_deref() == Some(repo_id.as_str());
    }

    signals.current_repo.get_untracked().as_deref() == Some(request.expected_name.as_str())
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
                .set(Some(PendingRepoSwitch::create(
                    target_repo.clone(),
                    switch_nonce,
                )));
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
                    .set(Some(PendingRepoSwitch::rename_current(
                        target_repo.clone(),
                        switch_nonce,
                    )));
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
                signals.set_pending_repo_switch.set(
                    request
                        .fallback_name
                        .clone()
                        .map(|name| PendingRepoSwitch::remove_current(name, switch_nonce)),
                );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ConnectionStatus, WsService};
    use crate::hooks::use_core::{PendingBranchSwitch, PendingRepoSwitch};
    use crate::runtime::document::pending::PendingLocalEdits;
    use deve_core::protocol::ClientMessage;
    use leptos::prelude::{GetUntracked, signal};

    #[test]
    fn switch_repo_exact_sends_repo_id_even_when_display_name_matches_current() {
        let owner = leptos::reactive::owner::Owner::new();
        owner.with(|| {
            let target_repo_id = RepoId::new_v4();
            let other_repo_id = RepoId::new_v4();
            let ws = WsService::new_for_test(ConnectionStatus::Connected);
            let (sync_banner, set_sync_banner) = signal(None::<String>);
            let signals = switch_signals(
                Some("display".to_string()),
                Some(other_repo_id.to_string()),
                7,
            );
            let callback = build_switch_repo_callback(ws.clone(), signals, set_sync_banner);

            callback.run(RepoSwitchRequest::exact(
                "display--id".to_string(),
                "display".to_string(),
                target_repo_id,
            ));

            assert_eq!(sync_banner.get_untracked(), None);
            let pending = signals
                .pending_repo_switch
                .get_untracked()
                .expect("pending repo switch");
            assert_eq!(pending.expected_name(), "display");
            assert_eq!(pending.switch_nonce, 8);
            match ws.drain_sent_for_test().as_slice() {
                [
                    ClientMessage::SwitchRepoExact {
                        name,
                        repo_id,
                        switch_nonce,
                    },
                ] => {
                    assert_eq!(name, "display--id");
                    assert_eq!(*repo_id, target_repo_id);
                    assert_eq!(*switch_nonce, Some(8));
                }
                other => panic!("expected one SwitchRepoExact message, got {other:?}"),
            }
        });
    }

    #[test]
    fn switch_repo_by_name_keeps_legacy_switch_repo_message() {
        let owner = leptos::reactive::owner::Owner::new();
        owner.with(|| {
            let ws = WsService::new_for_test(ConnectionStatus::Connected);
            let (_, set_sync_banner) = signal(None::<String>);
            let signals = switch_signals(Some("default".to_string()), None, 3);
            let callback = build_switch_repo_callback(ws.clone(), signals, set_sync_banner);

            callback.run(RepoSwitchRequest::by_name("legacy".to_string()));

            let pending = signals
                .pending_repo_switch
                .get_untracked()
                .expect("pending repo switch");
            assert_eq!(pending.expected_name(), "legacy");
            assert_eq!(pending.switch_nonce, 4);
            match ws.drain_sent_for_test().as_slice() {
                [ClientMessage::SwitchRepo { name, switch_nonce }] => {
                    assert_eq!(name, "legacy");
                    assert_eq!(*switch_nonce, Some(4));
                }
                other => panic!("expected one SwitchRepo message, got {other:?}"),
            }
        });
    }

    fn switch_signals(
        current_repo_value: Option<String>,
        current_repo_id_value: Option<String>,
        current_scope_nonce_value: u64,
    ) -> SwitchScopeSignals {
        let (current_doc, _) = signal(None);
        let (pending_local_edits, _) = signal(PendingLocalEdits::new());
        let (_, set_pending_navigation) = signal(None);
        let (current_repo, _) = signal(current_repo_value);
        let (current_repo_id, _) = signal(current_repo_id_value);
        let (current_scope_nonce, _) = signal(current_scope_nonce_value);
        let (active_branch, _) = signal(None);
        let (pending_branch_switch, set_pending_branch_switch) =
            signal(None::<PendingBranchSwitch>);
        let (pending_repo_switch, set_pending_repo_switch) = signal(None::<PendingRepoSwitch>);
        let (_, set_handshake_ready) = signal(true);
        let (_, set_handshake_scope_nonce) = signal(Some(current_scope_nonce_value));

        SwitchScopeSignals {
            current_doc,
            pending_local_edits,
            set_pending_navigation,
            current_repo,
            current_repo_id,
            current_scope_nonce,
            active_branch,
            pending_branch_switch,
            pending_repo_switch,
            set_handshake_ready,
            set_handshake_scope_nonce,
            set_pending_branch_switch,
            set_pending_repo_switch,
        }
    }
}
