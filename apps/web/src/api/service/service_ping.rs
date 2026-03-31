use super::ConnectionStatus;
use deve_core::protocol::ClientMessage;
use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;
use leptos::task::spawn_local;

pub(super) fn spawn_ping_loop(
    status: ReadSignal<ConnectionStatus>,
    tx: UnboundedSender<ClientMessage>,
) {
    let tx_check = StoredValue::new_local(Some(tx));
    let status_check = StoredValue::new_local(Some(status));

    spawn_local(async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(30_000).await;
            let Some(status_check) = status_check.try_get_value().flatten() else {
                break;
            };
            let Some(tx_check) = tx_check.try_get_value().flatten() else {
                break;
            };
            if status_check.get_untracked() == ConnectionStatus::Connected {
                let _ = tx_check.unbounded_send(ClientMessage::Ping);
            }
        }
    });

    on_cleanup(move || {
        status_check.update_value(|value| {
            let _ = value.take();
        });
        tx_check.update_value(|value| drop(value.take()));
    });
}
