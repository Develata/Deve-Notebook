use crate::api::WsService;
use leptos::prelude::{GetUntracked, Set};

use super::super::state::CoreSignals;
use super::message_scope::{RequestMatch, ShadowListScope, shadow_list_matches_scope};
use super::message_shadow;

pub fn handle_shadow_list_message(
    request_id: Option<String>,
    scope_nonce: Option<u64>,
    shadows: Vec<String>,
    ws: &WsService,
    signals: CoreSignals,
) {
    let scope = ShadowListScope {
        pending_branch_switch: signals.pending_branch_switch.get_untracked(),
        pending_repo_switch: signals.pending_repo_switch.get_untracked(),
    };
    let accepts_system_shadow = request_id.is_none()
        && shadow_list_matches_scope(
            RequestMatch {
                message_id: None,
                expected_id: None,
                scope_nonce,
                current_scope_nonce: signals.current_scope_nonce.get_untracked(),
            },
            &scope,
        );
    if accepts_system_shadow {
        signals.set_shadow_list_request_id.set(None);
        message_shadow::handle_shadow_list(shadows, true, ws, signals);
        return;
    }
    if shadow_list_matches_scope(
        RequestMatch {
            message_id: request_id.as_deref(),
            expected_id: signals.shadow_list_request_id.get_untracked().as_deref(),
            scope_nonce,
            current_scope_nonce: signals.current_scope_nonce.get_untracked(),
        },
        &scope,
    ) {
        if request_id.is_some() {
            signals.set_shadow_list_request_id.set(None);
        }
        message_shadow::handle_shadow_list(shadows, request_id.is_some(), ws, signals);
    }
}
