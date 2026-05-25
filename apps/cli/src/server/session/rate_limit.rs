//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Per-WebSocket message window accounting.

use super::WsSession;
use std::time::{Duration, Instant};

const WS_MESSAGE_WINDOW: Duration = Duration::from_secs(60);
const WS_MAX_MESSAGES_PER_WINDOW: u16 = 200;

impl WsSession {
    pub fn record_incoming_message(&mut self, now: Instant) -> bool {
        if now.duration_since(self.message_window_started_at) >= WS_MESSAGE_WINDOW {
            self.message_window_started_at = now;
            self.message_count_in_window = 0;
        }

        if self.message_count_in_window >= WS_MAX_MESSAGES_PER_WINDOW {
            return false;
        }

        self.message_count_in_window += 1;
        true
    }
}
