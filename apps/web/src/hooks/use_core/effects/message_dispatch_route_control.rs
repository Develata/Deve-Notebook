//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
use crate::api::WsService;
use deve_core::protocol::ServerMessage;

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
            repos,
        } => {
            handle_repo_list_message(request_id, branch, scope_nonce, repos, ws, signals);
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
            name,
            uuid,
            switch_nonce,
        } => {
            handle_repo_switched_message(branch, name, uuid, switch_nonce, ws, signals);
            None
        }
        ServerMessage::PeerDeleted {
            peer_id,
            scope_nonce,
        } => {
            handle_peer_deleted_message(peer_id, scope_nonce, ws, signals);
            None
        }
        other => Some(other),
    }
}
