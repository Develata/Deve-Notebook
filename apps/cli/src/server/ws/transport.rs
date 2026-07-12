//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 07_network#native-full-peer-runtime
//!
//! Per-listener ownership for upgraded WebSocket sessions.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{Notify, watch};

#[derive(Default)]
struct TransportState {
    stopping: bool,
    active_sessions: usize,
}

pub(crate) struct WsTransportRuntime {
    state: Mutex<TransportState>,
    shutdown: watch::Sender<bool>,
    idle: Notify,
}

pub(crate) struct WsTransportSessionPermit {
    runtime: Arc<WsTransportRuntime>,
}

impl WsTransportSessionPermit {
    pub(crate) fn subscribe(&self) -> watch::Receiver<bool> {
        self.runtime.subscribe()
    }
}

impl WsTransportRuntime {
    pub(crate) fn new() -> Arc<Self> {
        let (shutdown, _) = watch::channel(false);
        Arc::new(Self {
            state: Mutex::new(TransportState::default()),
            shutdown,
            idle: Notify::new(),
        })
    }

    pub(crate) fn reserve_session(self: &Arc<Self>) -> Option<WsTransportSessionPermit> {
        let mut state = self.state.lock().ok()?;
        if state.stopping {
            return None;
        }
        state.active_sessions = state.active_sessions.saturating_add(1);
        Some(WsTransportSessionPermit {
            runtime: self.clone(),
        })
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<bool> {
        self.shutdown.subscribe()
    }

    pub(crate) fn begin_shutdown(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.stopping = true;
        }
        let _ = self.shutdown.send(true);
        self.idle.notify_waiters();
    }

    pub(crate) async fn wait_for_idle(&self, timeout: Duration) -> anyhow::Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let notified = self.idle.notified();
            let active = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("WS transport state poisoned"))?
                .active_sessions;
            if active == 0 {
                return Ok(());
            }
            tokio::time::timeout_at(deadline, notified)
                .await
                .map_err(|_| anyhow::anyhow!("WS transport session shutdown timed out"))?;
        }
    }
}

impl Drop for WsTransportSessionPermit {
    fn drop(&mut self) {
        if let Ok(mut state) = self.runtime.state.lock() {
            state.active_sessions = state.active_sessions.saturating_sub(1);
        }
        self.runtime.idle.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_rejects_new_sessions_and_waits_for_existing_permit() {
        let runtime = WsTransportRuntime::new();
        let permit = runtime.reserve_session().expect("permit");
        runtime.begin_shutdown();
        assert!(runtime.reserve_session().is_none());
        drop(permit);
        runtime
            .wait_for_idle(Duration::from_secs(1))
            .await
            .expect("idle");
    }
}
