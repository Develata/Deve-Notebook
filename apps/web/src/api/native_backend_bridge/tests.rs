//! plan_ref:
//!   - 15_settings#native-host-local-backend-preference

fn source_before_tests() -> &'static str {
    include_str!("../native_backend_bridge.rs")
        .split("#[cfg(test)]")
        .next()
        .expect("source before tests")
}

#[test]
fn native_backend_facade_reads_through_bridge_registry() {
    let source = source_before_tests();

    assert!(source.contains("NATIVE_BACKEND_CONFIG_FACADE"));
    assert!(source.contains("\"__deveWebBridge\""));
    assert!(source.contains("\"get\""));
    assert!(
        source.contains("&JsValue::from_str(NATIVE_BACKEND_CONFIG_FACADE)"),
        "native backend config facade must be requested through the bridge registry"
    );
    assert!(
        source
            .match_indices("Reflect::get(window.as_ref(),")
            .all(|(index, _)| {
                let lookup_tail = &source[index..source.len().min(index + 240)];
                !lookup_tail.contains(super::NATIVE_BACKEND_CONFIG_FACADE)
                    && !lookup_tail.contains("__DEVE_NATIVE_BACKEND_CONFIG__")
            }),
        "native backend bridge must not read the facade directly from window"
    );
}

#[test]
fn native_backend_config_response_requires_structured_mode() {
    let local = super::parse_config_fields(Some("local".to_string()), None);
    assert!(local.available);
    assert_eq!(local.mode, "local");
    assert!(local.remote_url.is_empty());

    let remote = super::parse_config_fields(
        Some("remote".to_string()),
        Some("https://deve.example".to_string()),
    );
    assert!(remote.available);
    assert_eq!(remote.mode, "remote");
    assert_eq!(remote.remote_url, "https://deve.example");

    for config in [
        super::parse_config_fields(None, None),
        super::parse_config_fields(Some("".to_string()), None),
        super::parse_config_fields(Some("unexpected".to_string()), None),
        super::parse_config_fields(Some("local".to_string()), Some("https://stale".into())),
        super::parse_config_fields(Some("remote".to_string()), None),
        super::parse_config_fields(Some("remote".to_string()), Some("   ".into())),
    ] {
        assert!(!config.available);
        assert_eq!(
            config.error_message.as_deref(),
            Some(super::INVALID_NATIVE_BACKEND_RESPONSE)
        );
    }
}

#[test]
fn native_backend_validation_success_requires_origin_and_node_role() {
    let success = super::parse_validation_fields(
        true,
        Some("https://deve.example".to_string()),
        Some("native-main".to_string()),
        None,
    );
    assert!(success.available);
    assert!(success.ok);
    assert_eq!(success.https_origin, "https://deve.example");
    assert_eq!(success.node_role, "native-main");

    let failed_probe =
        super::parse_validation_fields(false, None, None, Some("probe failed".to_string()));
    assert!(failed_probe.available);
    assert!(!failed_probe.ok);
    assert_eq!(failed_probe.error_message.as_deref(), Some("probe failed"));

    for validation in [
        super::parse_validation_fields(true, None, Some("native-main".into()), None),
        super::parse_validation_fields(true, Some("https://deve.example".into()), None, None),
        super::parse_validation_fields(true, Some("   ".into()), Some("native-main".into()), None),
        super::parse_validation_fields(
            true,
            Some("https://deve.example".into()),
            Some("   ".into()),
            None,
        ),
    ] {
        assert!(!validation.available);
        assert!(!validation.ok);
        assert_eq!(
            validation.error_message.as_deref(),
            Some(super::INVALID_NATIVE_BACKEND_RESPONSE)
        );
    }
}
