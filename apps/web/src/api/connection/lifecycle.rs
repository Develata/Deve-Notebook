//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 09_auth#unauthorized-disconnected-ui
//!

use leptos::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
pub(crate) struct ConnectionLifecycle {
    active: Arc<AtomicBool>,
}

impl ConnectionLifecycle {
    pub(crate) fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(true)),
        }
    }

    pub(crate) fn shutdown(&self) {
        self.active.store(false, Ordering::Release);
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub(crate) fn try_get<T>(&self, signal: ReadSignal<T>) -> Option<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.is_active()
            .then(|| signal.try_get_untracked())
            .flatten()
    }

    pub(crate) fn try_set<T>(&self, signal: WriteSignal<T>, value: T) -> bool
    where
        T: Send + Sync + 'static,
    {
        self.is_active() && signal.try_set(value).is_none()
    }
}
