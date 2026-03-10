use crate::api::WsService;
use crate::i18n::Locale;
use deve_core::protocol::ClientMessage;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use super::super::state::CoreSignals;
use super::message_dispatch;

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
                if let Some(t) = changes_refresh.borrow_mut().take() {
                    t.cancel();
                }
                let ws_for_timer = ws.clone();
                let timer = Timeout::new(120, move || {
                    ws_for_timer.send(ClientMessage::GetChanges);
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
