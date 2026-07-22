//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
use crate::api::WsService;
use crate::i18n::{Locale, t};
use crate::runtime::repo_control_client::{
    RepoControlAdmission, RepoControlClient, RepoControlScope,
};
use deve_core::protocol::{
    RepoLifecycleOperation, RepoLifecycleOutcome, ServerErrorCode, ServerMessage,
};
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::state::CoreSignals;
use super::message_dispatch_control::{
    handle_branch_switched_message, handle_peer_deleted_message, handle_repo_list_message,
    handle_repo_switched_message,
};
use super::message_dispatch_shadow::handle_shadow_list_message;

pub fn route_control_message(
    msg: ServerMessage,
    ws: &WsService,
    signals: CoreSignals,
    locale: Locale,
    repo_control: &RepoControlClient,
) -> Option<ServerMessage> {
    match msg {
        ServerMessage::ShadowList {
            request_id,
            scope_nonce,
            shadows,
        } => {
            handle_shadow_list_message(request_id, scope_nonce, shadows, ws, signals);
            None
        }
        ServerMessage::RepoList {
            request_id,
            branch,
            scope_nonce,
            repo_entries,
        } => {
            handle_repo_list_message(request_id, branch, scope_nonce, repo_entries, ws, signals);
            repo_control.resume_lifecycles(ws, current_repo_control_scope(ws, signals));
            None
        }
        ServerMessage::BranchSwitched {
            peer_id,
            success,
            switch_nonce,
        } => {
            handle_branch_switched_message(peer_id, success, switch_nonce, ws, signals);
            None
        }
        ServerMessage::RepoSwitched {
            branch,
            repo_id,
            display_alias,
            switch_nonce,
            scope_nonce,
        } => {
            if switch_nonce != Some(scope_nonce.get()) {
                leptos::logging::warn!("忽略 RepoSwitched: switch_nonce 与 scope_nonce 不匹配");
                return None;
            }
            handle_repo_switched_message(
                branch,
                display_alias,
                repo_id.to_string(),
                switch_nonce,
                ws,
                signals,
            );
            repo_control.resume_lifecycles(ws, current_repo_control_scope(ws, signals));
            None
        }
        ServerMessage::PeerDeleted {
            peer_id,
            scope_nonce,
        } => {
            handle_peer_deleted_message(peer_id, scope_nonce, ws, signals);
            None
        }
        ServerMessage::RepoControl(response) => {
            let scope = current_repo_control_scope(ws, signals);
            if let Some(admission) = repo_control.accept(response, &scope) {
                match admission {
                    RepoControlAdmission::AliasSet(_) => {}
                    RepoControlAdmission::RemovalPrepared { .. } => {}
                    RepoControlAdmission::LifecycleAccepted {
                        target_repo_id,
                        operation: RepoLifecycleOperation::Create,
                        ..
                    } => {
                        let mut bound = false;
                        signals.set_pending_repo_switch.update(|pending| {
                            bound = pending
                                .as_mut()
                                .is_some_and(|pending| pending.bind_created_repo(target_repo_id));
                        });
                        if !bound {
                            signals.set_pending_repo_switch.set(None);
                            signals.set_sync_banner.set(Some(
                                t::server_error::message(
                                    locale,
                                    ServerErrorCode::RepoLifecycleInvalidRequest,
                                )
                                .to_string(),
                            ));
                        }
                    }
                    RepoControlAdmission::LifecycleAccepted { .. } => {}
                    RepoControlAdmission::LifecycleStatus {
                        outcome,
                        publication_pending,
                        ..
                    } => {
                        let code = if publication_pending {
                            Some(ServerErrorCode::RepoLifecyclePublicationPending)
                        } else {
                            match outcome {
                                Some(RepoLifecycleOutcome::CommittedPartial) => {
                                    Some(ServerErrorCode::RepoLifecycleCommittedPartial)
                                }
                                Some(RepoLifecycleOutcome::RepairRequired) => {
                                    Some(ServerErrorCode::RepoLifecycleRepairRequired)
                                }
                                _ => None,
                            }
                        };
                        if let Some(code) = code {
                            signals
                                .set_sync_banner
                                .set(Some(t::server_error::message(locale, code).to_string()));
                        }
                    }
                    RepoControlAdmission::Error {
                        code,
                        lifecycle_request,
                    } => {
                        signals
                            .set_sync_banner
                            .set(Some(t::server_error::message(locale, code).to_string()));
                        if lifecycle_request {
                            signals.set_pending_repo_switch.set(None);
                            signals
                                .set_handshake_retry_nonce
                                .update(|nonce| *nonce = nonce.wrapping_add(1));
                        }
                    }
                }
            }
            None
        }
        other => Some(other),
    }
}

fn current_repo_control_scope(ws: &WsService, signals: CoreSignals) -> RepoControlScope {
    RepoControlScope::new(
        ws.connection_epoch.get_untracked(),
        signals
            .current_repo_id
            .get_untracked()
            .and_then(|repo_id| repo_id.parse().ok()),
        signals
            .active_branch
            .get_untracked()
            .map(|branch| branch.to_string()),
        signals.current_scope_nonce.get_untracked(),
    )
}
