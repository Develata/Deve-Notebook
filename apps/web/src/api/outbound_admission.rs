//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
//! Bounded UI-intent admission for the browser connection manager.
//!
//! `futures::mpsc::Sender` adds one reserved slot per sender clone. The runtime
//! therefore owns exactly one sender inside `Arc<Mutex<_>>`; cloned handles
//! share that sender instead of cloning it and silently raising capacity.
//! Frames also retain a shared permit while moving through the session queue,
//! so draining the mpsc channel cannot evade the end-to-end count/byte limits.

use deve_core::protocol::ClientMessage;
use deve_core::protocol::frame::encode_client_binary;
use futures::channel::mpsc::{Receiver, Sender, channel};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::output::{OutboundMessageClass, classify_outbound_message};

pub(super) const OUTBOUND_ADMISSION_LIMIT: usize = 500;
pub(super) const OUTBOUND_ADMISSION_BYTES_LIMIT: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OutboundAdmissionFailureKind {
    Encode,
    Saturated,
    Closed,
}

impl OutboundAdmissionFailureKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Encode => "encode",
            Self::Saturated => "saturated",
            Self::Closed => "closed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct OutboundAdmissionFailure {
    pub(super) kind: OutboundAdmissionFailureKind,
    pub(super) message_class: OutboundMessageClass,
}

struct OutboundBudget {
    count_limit: usize,
    bytes_limit: usize,
    count: AtomicUsize,
    bytes: AtomicUsize,
}

impl OutboundBudget {
    fn reserve(self: &Arc<Self>, bytes: usize) -> Option<OutboundPermit> {
        // These counters guard quota only; the channel and sender mutex own
        // frame visibility, so no cross-object memory ordering is required.
        let count_reserved = self
            .count
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                (count < self.count_limit).then_some(count + 1)
            })
            .is_ok();
        if !count_reserved {
            return None;
        }
        let bytes_reserved = self
            .bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current
                    .checked_add(bytes)
                    .filter(|next| *next <= self.bytes_limit)
            })
            .is_ok();
        if !bytes_reserved {
            self.count.fetch_sub(1, Ordering::Relaxed);
            return None;
        }
        Some(OutboundPermit {
            budget: Arc::clone(self),
            bytes,
        })
    }
}

struct OutboundPermit {
    budget: Arc<OutboundBudget>,
    bytes: usize,
}

impl Drop for OutboundPermit {
    fn drop(&mut self) {
        self.budget.bytes.fetch_sub(self.bytes, Ordering::Relaxed);
        self.budget.count.fetch_sub(1, Ordering::Relaxed);
    }
}

pub(super) struct OutboundFrame {
    bytes: Vec<u8>,
    message_class: OutboundMessageClass,
    _permit: Option<OutboundPermit>,
}

impl fmt::Debug for OutboundFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OutboundFrame")
            .field("byte_len", &self.bytes.len())
            .field("message_class", &self.message_class)
            .finish_non_exhaustive()
    }
}

impl OutboundFrame {
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(super) const fn message_class(&self) -> OutboundMessageClass {
        self.message_class
    }

    pub(super) fn system_ping() -> Result<Self, OutboundAdmissionFailure> {
        Self::encode_without_permit(ClientMessage::Ping)
    }

    #[cfg(test)]
    pub(crate) fn for_test(message: ClientMessage) -> Self {
        Self::encode_without_permit(message).expect("test client frame must encode")
    }

    fn encode_without_permit(message: ClientMessage) -> Result<Self, OutboundAdmissionFailure> {
        let (bytes, message_class) = encode_message(message)?;
        Ok(Self {
            bytes,
            message_class,
            _permit: None,
        })
    }
}

#[derive(Clone)]
pub(super) struct OutboundAdmissionSender {
    inner: Arc<Mutex<Sender<OutboundFrame>>>,
    budget: Arc<OutboundBudget>,
}

impl OutboundAdmissionSender {
    pub(super) fn try_admit(&self, message: ClientMessage) -> Result<(), OutboundAdmissionFailure> {
        let (bytes, message_class) = encode_message(message)?;
        let permit = self
            .budget
            .reserve(bytes.len())
            .ok_or(OutboundAdmissionFailure {
                kind: OutboundAdmissionFailureKind::Saturated,
                message_class,
            })?;
        let frame = OutboundFrame {
            bytes,
            message_class,
            _permit: Some(permit),
        };
        let mut sender = self.inner.lock().map_err(|_| OutboundAdmissionFailure {
            kind: OutboundAdmissionFailureKind::Closed,
            message_class,
        })?;
        sender
            .try_send(frame)
            .map_err(|error| OutboundAdmissionFailure {
                kind: if error.is_full() {
                    OutboundAdmissionFailureKind::Saturated
                } else {
                    OutboundAdmissionFailureKind::Closed
                },
                message_class,
            })
    }
}

fn encode_message(
    message: ClientMessage,
) -> Result<(Vec<u8>, OutboundMessageClass), OutboundAdmissionFailure> {
    let message_class = classify_outbound_message(&message);
    let bytes = encode_client_binary(&message).map_err(|_| OutboundAdmissionFailure {
        kind: OutboundAdmissionFailureKind::Encode,
        message_class,
    })?;
    drop(message);
    Ok((bytes, message_class))
}

pub(super) fn outbound_channel() -> (OutboundAdmissionSender, Receiver<OutboundFrame>) {
    outbound_channel_with_limits(OUTBOUND_ADMISSION_LIMIT, OUTBOUND_ADMISSION_BYTES_LIMIT)
}

fn outbound_channel_with_limits(
    count_limit: usize,
    bytes_limit: usize,
) -> (OutboundAdmissionSender, Receiver<OutboundFrame>) {
    assert!(count_limit > 0, "outbound admission limit must be positive");
    assert!(bytes_limit > 0, "outbound byte limit must be positive");
    // futures::mpsc capacity = buffer + sender count. This module owns one
    // underlying sender, so `count_limit - 1` buffer slots are exact.
    let (sender, receiver) = channel(count_limit - 1);
    (
        OutboundAdmissionSender {
            inner: Arc::new(Mutex::new(sender)),
            budget: Arc::new(OutboundBudget {
                count_limit,
                bytes_limit,
                count: AtomicUsize::new(0),
                bytes: AtomicUsize::new(0),
            }),
        },
        receiver,
    )
}

#[cfg(test)]
mod tests;
