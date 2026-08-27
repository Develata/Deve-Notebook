//! plan_ref:
//!   - 07_network#web-ws-runtime
//!

use super::ConnectionStatus;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::WsService;

pub(super) fn spawn_ping_loop(status: ReadSignal<ConnectionStatus>, service: WsService) {
    let service_check = StoredValue::new_local(Some(service));
    let status_check = StoredValue::new_local(Some(status));

    spawn_local(async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(30_000).await;
            let Some(status_check) = status_check.try_get_value().flatten() else {
                break;
            };
            let Some(service_check) = service_check.try_get_value().flatten() else {
                break;
            };
            if status_check.get_untracked() == ConnectionStatus::Connected {
                service_check.send(ClientMessage::Ping);
            }
        }
    });

    on_cleanup(move || {
        status_check.update_value(|value| {
            let _ = value.take();
        });
        service_check.update_value(|value| drop(value.take()));
    });
}
