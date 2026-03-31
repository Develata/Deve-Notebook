pub(super) fn can_send_delta(
    is_playback: bool,
    branch_switch_pending: bool,
    repo_switch_pending: bool,
    handshake_ready: bool,
    writer_ready: bool,
) -> bool {
    !is_playback
        && !branch_switch_pending
        && !repo_switch_pending
        && handshake_ready
        && writer_ready
}

#[cfg(test)]
mod tests {
    use super::can_send_delta;

    #[test]
    fn blocks_delta_while_scope_switch_is_pending() {
        assert!(!can_send_delta(false, true, false, true, true));
        assert!(!can_send_delta(false, false, true, true, true));
    }

    #[test]
    fn blocks_delta_before_handshake_is_ready() {
        assert!(!can_send_delta(false, false, false, false, true));
    }

    #[test]
    fn allows_delta_only_in_stable_writable_scope() {
        assert!(can_send_delta(false, false, false, true, true));
        assert!(!can_send_delta(false, false, false, true, false));
    }
}
