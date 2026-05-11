use super::*;

#[test]
fn doc_diff_accepts_matching_request_or_system_diff() {
    assert!(doc_diff_matches_request(
        &Some("req-1".into()),
        Some("req-1".into()),
        Some(3),
        3,
    ));
    assert!(!doc_diff_matches_request(
        &Some("stale".into()),
        Some("req-1".into()),
        Some(3),
        3,
    ));
    assert!(doc_diff_matches_request(&None, None, Some(3), 3));
    assert!(!doc_diff_matches_request(
        &None,
        Some("req-1".into()),
        Some(3),
        3
    ));
    assert!(!doc_diff_matches_request(&None, None, Some(2), 3));
}

#[test]
fn commit_diff_requires_matching_request_id() {
    assert!(commit_diff_matches_request(
        &Some("req-1".into()),
        Some("req-1".into()),
        Some(3),
        3,
    ));
    assert!(!commit_diff_matches_request(
        &Some("stale".into()),
        Some("req-1".into()),
        Some(3),
        3,
    ));
    assert!(!commit_diff_matches_request(
        &None,
        Some("req-1".into()),
        Some(3),
        3
    ));
}

#[test]
fn changes_and_history_require_matching_request_id() {
    assert!(changes_list_matches_request(
        &Some("req-1".into()),
        Some("req-1".into()),
        Some(3),
        3,
    ));
    assert!(!changes_list_matches_request(
        &Some("stale".into()),
        Some("req-1".into()),
        Some(3),
        3,
    ));
    assert!(changes_list_matches_request(&None, None, Some(3), 3));
    assert!(!changes_list_matches_request(
        &None,
        Some("req-1".into()),
        Some(3),
        3
    ));

    assert!(commit_history_matches_request(
        &Some("req-1".into()),
        Some("req-1".into()),
        Some(3),
        3,
    ));
    assert!(!commit_history_matches_request(
        &Some("stale".into()),
        Some("req-1".into()),
        Some(3),
        3,
    ));
    assert!(!commit_history_matches_request(
        &None,
        Some("req-1".into()),
        Some(3),
        3
    ));
}

#[test]
fn scoped_ack_requires_current_scope_nonce() {
    assert!(scoped_ack_matches(Some(3), 3));
    assert!(!scoped_ack_matches(Some(2), 3));
    assert!(!scoped_ack_matches(None, 3));
}
