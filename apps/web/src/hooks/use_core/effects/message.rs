use crate::api::WsService;
use crate::i18n::Locale;
use deve_core::protocol::ClientMessage;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use super::super::state::CoreSignals;
use super::message_dispatch;
use super::message_refresh::{can_issue_sc_refresh, capture_refresh_scope, should_send_refresh};

/// 设置消息处理 Effect。
pub fn setup(ws: &WsService, signals: &CoreSignals) {
    let ws_rx = ws.clone();
    let signals = *signals;
    let degraded_sync_mode = signals.degraded_sync_mode;
    let set_sync_banner = signals.set_sync_banner;
    let changes_refresh = Rc::new(RefCell::new(None::<Timeout>));
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
                let changes_request_id = signals.changes_request_id;
                let set_changes_request_id = signals.set_changes_request_id;
                let current_repo_id = signals.current_repo_id;
                let active_branch = signals.active_branch;
                let pending_branch_switch = signals.pending_branch_switch;
                let pending_repo_switch = signals.pending_repo_switch;
                let timer = Timeout::new(120, move || {
                    if !should_send_refresh(
                        &refresh_scope,
                        current_repo_id.get_untracked(),
                        active_branch.get_untracked(),
                        pending_branch_switch.get_untracked(),
                        pending_repo_switch.get_untracked(),
                        signals.current_scope_nonce.get_untracked(),
                    ) {
                        return;
                    }
                    if !can_issue_sc_refresh(changes_request_id.get_untracked()) {
                        return;
                    }
                    let request_id = uuid::Uuid::new_v4().to_string();
                    set_changes_request_id.set(Some(request_id.clone()));
                    ws_for_timer.send(ClientMessage::GetChanges { request_id });
                });
                *changes_refresh.borrow_mut() = Some(timer);
            }
        };

        if let Some(msg) = ws_rx.msg.get() {
            message_dispatch::handle_message(
                msg,
                &ws_rx,
                signals,
                locale.get_untracked(),
                &schedule_refresh,
            );
        }
    });
}
