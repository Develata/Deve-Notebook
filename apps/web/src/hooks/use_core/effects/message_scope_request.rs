//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
#[derive(Clone, Copy)]
pub struct RequestMatch<'a> {
    pub message_id: Option<&'a str>,
    pub expected_id: Option<&'a str>,
    pub scope_nonce: Option<u64>,
    pub current_scope_nonce: u64,
}

pub fn accepts_system_or_matching_request(
    message_id: Option<&str>,
    expected_id: Option<&str>,
    scope_nonce: Option<u64>,
    current_scope_nonce: u64,
) -> bool {
    request_matches(RequestMatch {
        message_id,
        expected_id,
        scope_nonce,
        current_scope_nonce,
    })
}

pub fn request_matches(request: RequestMatch<'_>) -> bool {
    let scoped_match = request.scope_nonce == Some(request.current_scope_nonce);
    match request.message_id {
        Some(message_id) => request.expected_id == Some(message_id) && scoped_match,
        None => request.expected_id.is_none() && scoped_match,
    }
}
