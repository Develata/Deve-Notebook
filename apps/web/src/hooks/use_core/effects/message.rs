//! plan_ref:
//!   - 05_network#web-ws-runtime
//!
use crate::api::{WsService, is_current_connection_message};
use crate::i18n::Locale;
use deve_core::protocol::ClientMessage;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use super::super::state::CoreSignals;
use super::super::write_gate::RepoWriteGateState;
use super::message_dispatch;
use super::message_refresh::{capture_refresh_scope, should_send_refresh_through_read_gate};

/// 设置消息处理 Effect。
pub fn setup(ws: &WsService, signals: &CoreSignals) {
    let ws_rx = ws.clone();
    let signals = *signals;
    let degraded_sync_mode = signals.degraded_sync_mode;
    let set_sync_banner = signals.set_sync_banner;
    let changes_refresh = Rc::new(RefCell::new(None::<Timeout>));
    let (last_msg_seq, set_last_msg_seq) = signal(0u64);
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    Effect::new(move |_| {
        let banner = degraded_sync_mode
            .get()
            .map(|mode| format!("存储受限（{}），当前处于只读模式", mode.reason));
        set_sync_banner.set(banner);
    });

    Effect::new(move |_| {
        let schedule_refresh = {
            let changes_refresh = changes_refresh.clone();
            let ws = ws_rx.clone();
            move || {
                let refresh_scope = capture_refresh_scope(
                    signals.current_repo_id.get_untracked(),
                    signals.active_branch.get_untracked(),
                    signals.pending_branch_switch.get_untracked(),
                    signals.pending_repo_switch.get_untracked(),
                    signals.current_scope_nonce.get_untracked(),
                );
                let Some(refresh_scope) = refresh_scope else {
                    return;
                };
                if let Some(t) = changes_refresh.borrow_mut().take() {
                    t.cancel();
                }
                let ws_for_timer = ws.clone();
                let set_changes_request_id = signals.set_changes_request_id;
                let current_repo_id = signals.current_repo_id;
                let active_branch = signals.active_branch;
                let pending_branch_switch = signals.pending_branch_switch;
                let pending_repo_switch = signals.pending_repo_switch;
                let timer = Timeout::new(120, move || {
                    let repo_id = current_repo_id.get_untracked();
                    let branch = active_branch.get_untracked();
                    let pending_branch = pending_branch_switch.get_untracked();
                    let pending_repo = pending_repo_switch.get_untracked();
                    let scope_nonce = signals.current_scope_nonce.get_untracked();
                    let load_state = signals.load_state.get_untracked();
                    if !should_send_refresh_through_read_gate(
                        &refresh_scope,
                        repo_id.clone(),
                        branch,
                        pending_branch.clone(),
                        pending_repo.clone(),
                        scope_nonce,
                        RepoWriteGateState {
                            connection_status: ws_for_timer.status.get_untracked(),
                            load_state: &load_state,
                            is_read_only: signals.is_spectator.get_untracked(),
                            handshake_ready: signals.handshake_ready.get_untracked(),
                            writer_ready: ws_for_timer
                                .writer_ready_for(repo_id.as_deref(), Some(scope_nonce)),
                            has_repo: repo_id.is_some(),
                            pending_branch_switch: pending_branch.is_some(),
                            pending_repo_switch: pending_repo.is_some(),
                        },
                    ) {
                        return;
                    }
                    let request_id = uuid::Uuid::new_v4().to_string();
                    set_changes_request_id.set(Some(request_id.clone()));
                    ws_for_timer.send(ClientMessage::GetChanges {
                        request_id,
                        scope_nonce: Some(scope_nonce),
                    });
                });
                *changes_refresh.borrow_mut() = Some(timer);
            }
        };

        let _ = ws_rx.msg_seq.get();
        for (seq, connection_epoch, msg) in ws_rx.messages_since(last_msg_seq.get_untracked()) {
            if !is_current_connection_message(
                connection_epoch,
                ws_rx.connection_epoch.get_untracked(),
            ) {
                set_last_msg_seq.set(seq);
                continue;
            }
            message_dispatch::handle_message(
                msg,
                &ws_rx,
                signals,
                locale.get_untracked(),
                &schedule_refresh,
            );
            set_last_msg_seq.set(seq);
        }
    });
}
