//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 18_release#runtime-observability
//!

use super::super::{http_base_from_ws_url, node_role_url_for_http_base};

#[test]
fn derives_http_base_from_ws_url() {
    assert_eq!(
        http_base_from_ws_url("ws://127.0.0.1:3001/ws"),
        "http://127.0.0.1:3001"
    );
    assert_eq!(
        http_base_from_ws_url("wss://example.test/ws"),
        "https://example.test"
    );
}

#[test]
fn ws_url_to_http_base_only_rewrites_leading_scheme_and_ws_suffix() {
    assert_eq!(
        http_base_from_ws_url("ws://127.0.0.1:3001/ws?next=ws://shadow/ws"),
        "http://127.0.0.1:3001"
    );
    assert_eq!(
        http_base_from_ws_url("custom://127.0.0.1:3001/ws"),
        "custom://127.0.0.1:3001"
    );
}

#[test]
fn node_role_probe_url_never_appends_after_query_or_fragment() {
    assert_eq!(
        node_role_url_for_http_base("http://127.0.0.1:3001?next=/ws"),
        "http://127.0.0.1:3001/api/node/role"
    );
    assert_eq!(
        node_role_url_for_http_base("http://127.0.0.1:3001/#/doc"),
        "http://127.0.0.1:3001/api/node/role"
    );
}
