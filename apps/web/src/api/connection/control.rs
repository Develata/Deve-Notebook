//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
//! Connection-manager control intents and bounded wait/retirement helpers.

use futures::channel::mpsc::{Receiver, UnboundedReceiver};
use futures::{FutureExt, StreamExt};
use gloo_timers::future::TimeoutFuture;
use std::collections::VecDeque;

use crate::api::backoff::BackoffStrategy;
use crate::api::outbound_admission::OutboundFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionControl {
    RebindNativeEndpoint,
    ReconnectForResync { connection_epoch: u64 },
    RetireOutboundAdmission { observed_connection_epoch: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BackoffWait {
    Elapsed,
    Rebind,
    RetireOutboundAdmission,
    Closed,
}

pub(super) async fn wait_for_rebind(
    control_rx: &mut UnboundedReceiver<ConnectionControl>,
    rx: &mut Receiver<OutboundFrame>,
    queue: &mut VecDeque<OutboundFrame>,
) -> bool {
    loop {
        match control_rx.next().await {
            Some(ConnectionControl::RebindNativeEndpoint) => return true,
            Some(ConnectionControl::ReconnectForResync { .. }) => continue,
            Some(ConnectionControl::RetireOutboundAdmission { .. }) => {
                retire_failed_generation_queues(rx, queue);
            }
            None => return false,
        }
    }
}

pub(super) async fn wait_for_backoff_or_rebind(
    backoff: &mut BackoffStrategy,
    control_rx: &mut UnboundedReceiver<ConnectionControl>,
) -> BackoffWait {
    let delay = backoff.take_delay_ms();
    leptos::logging::log!("WS: Reconnecting in {}ms...", delay);
    let timer = TimeoutFuture::new(delay).fuse();
    let control = control_rx.next().fuse();
    futures::pin_mut!(timer, control);
    futures::select! {
        _ = timer => BackoffWait::Elapsed,
        command = control => match command {
            Some(ConnectionControl::RebindNativeEndpoint) => BackoffWait::Rebind,
            Some(ConnectionControl::ReconnectForResync { .. }) => BackoffWait::Elapsed,
            Some(ConnectionControl::RetireOutboundAdmission { .. }) => {
                BackoffWait::RetireOutboundAdmission
            }
            None => BackoffWait::Closed,
        },
    }
}

pub(super) fn retire_failed_generation_queues(
    rx: &mut Receiver<OutboundFrame>,
    queue: &mut VecDeque<OutboundFrame>,
) -> (usize, usize) {
    let session_queue_len = queue.len();
    queue.clear();
    let mut admission_queue_len = 0usize;
    while rx.try_recv().is_ok() {
        admission_queue_len = admission_queue_len.saturating_add(1);
    }
    leptos::logging::warn!(
        "web_outbound_failed_generation_retired admission_count={} session_count={}",
        admission_queue_len,
        session_queue_len
    );
    (admission_queue_len, session_queue_len)
}
