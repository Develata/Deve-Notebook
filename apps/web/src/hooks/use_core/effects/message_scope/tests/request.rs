use super::*;

#[test]
fn system_or_matching_request_accepts_none_and_exact_match() {
    assert!(accepts_system_or_matching_request(None, None, Some(3), 3));
    assert!(!accepts_system_or_matching_request(
        None,
        Some("req-1"),
        Some(3),
        3,
    ));
    assert!(accepts_system_or_matching_request(
        Some("req-1"),
        Some("req-1"),
        Some(3),
        3,
    ));
    assert!(!accepts_system_or_matching_request(
        Some("stale"),
        Some("req-1"),
        Some(7),
        3,
    ));
    assert!(!accepts_system_or_matching_request(None, None, Some(2), 3));
    assert!(!accepts_system_or_matching_request(None, None, None, 3));
}

#[test]
fn requested_message_without_scope_nonce_is_rejected() {
    assert!(!accepts_system_or_matching_request(
        Some("req-1"),
        Some("req-1"),
        None,
        3,
    ));
}
