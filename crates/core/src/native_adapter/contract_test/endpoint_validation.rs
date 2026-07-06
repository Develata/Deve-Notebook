use super::endpoint;
use crate::native_adapter::{
    NativeAdapterError, validate_native_endpoint_bases, validate_native_endpoint_ready,
};

#[test]
fn native_endpoint_validation_accepts_loopback_bases() {
    let endpoint = endpoint("http://127.0.0.1:3001", "ws://localhost:3001", true);

    assert_eq!(validate_native_endpoint_ready(&endpoint), Ok(()));
}

#[test]
fn native_endpoint_validation_rejects_non_loopback_hosts() {
    let endpoint = endpoint("http://192.168.1.10:3001", "ws://127.0.0.1:3001", true);

    assert!(matches!(
        validate_native_endpoint_ready(&endpoint),
        Err(NativeAdapterError::NonLoopbackHost { field: "http_base" })
    ));
}

#[test]
fn native_endpoint_validation_rejects_scan_like_host_suffixes() {
    let endpoint = endpoint(
        "http://127.0.0.1.evil.example:3001",
        "ws://127.0.0.1:3001",
        true,
    );

    assert!(matches!(
        validate_native_endpoint_ready(&endpoint),
        Err(NativeAdapterError::NonLoopbackHost { field: "http_base" })
    ));
}

#[test]
fn native_endpoint_validation_rejects_url_credentials() {
    let endpoint = endpoint("http://token@127.0.0.1:3001", "ws://127.0.0.1:3001", true);

    assert!(matches!(
        validate_native_endpoint_ready(&endpoint),
        Err(NativeAdapterError::UserInfoForbidden { field: "http_base" })
    ));
}

#[test]
fn native_endpoint_validation_rejects_invalid_or_zero_ports() {
    for port in ["0", "65536", "not-a-port", ""] {
        let endpoint = endpoint(
            &format!("http://127.0.0.1:{port}"),
            "ws://127.0.0.1:3001",
            true,
        );

        assert!(matches!(
            validate_native_endpoint_ready(&endpoint),
            Err(NativeAdapterError::InvalidPort { field: "http_base" })
        ));
    }
}

#[test]
fn native_endpoint_ready_requires_session_binding() {
    let endpoint = endpoint("http://127.0.0.1:3001", "ws://127.0.0.1:3001", false);

    assert_eq!(validate_native_endpoint_bases(&endpoint), Ok(()));
    assert_eq!(
        validate_native_endpoint_ready(&endpoint),
        Err(NativeAdapterError::SessionNotBound)
    );
}
