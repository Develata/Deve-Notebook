//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
pub(super) fn can_send_delta(is_playback: bool, write_allowed: bool) -> bool {
    !is_playback && write_allowed
}

#[cfg(test)]
mod tests {
    use super::can_send_delta;

    #[test]
    fn blocks_delta_while_playback_is_active() {
        assert!(!can_send_delta(true, true));
    }

    #[test]
    fn blocks_delta_when_runtime_write_gate_blocks() {
        assert!(!can_send_delta(false, false));
    }

    #[test]
    fn allows_delta_only_when_playback_is_inactive_and_write_gate_allows() {
        assert!(can_send_delta(false, true));
    }
}
