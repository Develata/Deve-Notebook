use deve_core::native_adapter::{
    NativeProcessRuntimeFailureKind, NativeProcessRuntimeState, NativeServiceHealthProbe,
};
use serde_json::json;

use crate::{
    DesktopLocalServiceBootstrapError, DesktopLocalServiceProbeOutcome, DesktopLocalServiceRuntime,
    DesktopShell, node_role_probe_outcome_from_json, run_desktop_local_service_bootstrap,
    session_material_from_auth_status_json,
};

use super::support::{
    FakeLauncher, FakeProbe, FakeSessionHandoff, enabled_policy, endpoint, healthy_probe, plan,
};

#[test]
fn desktop_local_service_bootstrap_requires_probe_and_session_before_script() {
    let plan = plan();
    let mut runtime =
        DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, FakeLauncher::default());
    let mut shell = DesktopShell::new();
    let mut probe = FakeProbe {
        outcome: DesktopLocalServiceProbeOutcome {
            endpoint: endpoint(false),
            probe: healthy_probe(),
        },
    };
    let mut handoff = FakeSessionHandoff {
        session_bound: true,
    };

    let result = run_desktop_local_service_bootstrap(
        &plan,
        &mut runtime,
        &mut shell,
        &mut probe,
        &mut handoff,
        10,
    )
    .expect("bootstrap");

    assert_eq!(result.bootstrap.http_base, "http://127.0.0.1:39101");
    assert_eq!(result.bootstrap.ws_base, "ws://127.0.0.1:39101");
    assert!(result.bootstrap.session_bound);
    assert!(
        result
            .bootstrap_script
            .contains("window.__DEVE_NATIVE_BOOTSTRAP")
    );
    assert_eq!(
        result.runtime_snapshot.state,
        NativeProcessRuntimeState::SessionHandoffReady
    );
    assert!(!result.runtime_snapshot.authority_writes_allowed);
    assert!(shell.snapshot().endpoint.expect("endpoint").session_bound);
}

#[test]
fn desktop_local_service_bootstrap_blocks_unhealthy_probe() {
    let plan = plan();
    let mut runtime =
        DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, FakeLauncher::default());
    let mut shell = DesktopShell::new();
    let mut probe = FakeProbe {
        outcome: DesktopLocalServiceProbeOutcome {
            endpoint: endpoint(false),
            probe: NativeServiceHealthProbe::default(),
        },
    };
    let mut handoff = FakeSessionHandoff {
        session_bound: true,
    };

    let error = run_desktop_local_service_bootstrap(
        &plan,
        &mut runtime,
        &mut shell,
        &mut probe,
        &mut handoff,
        10,
    )
    .expect_err("probe failure blocks bootstrap");

    assert!(matches!(
        error,
        DesktopLocalServiceBootstrapError::HealthProbeFailed
    ));
    assert!(shell.recovery_bootstrap_for_web().is_some());
    assert!(!runtime.snapshot().authority_writes_allowed);
}

#[test]
fn desktop_local_service_bootstrap_blocks_session_handoff_failure() {
    let plan = plan();
    let mut runtime =
        DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, FakeLauncher::default());
    let mut shell = DesktopShell::new();
    let mut probe = FakeProbe {
        outcome: DesktopLocalServiceProbeOutcome {
            endpoint: endpoint(false),
            probe: healthy_probe(),
        },
    };
    let mut handoff = FakeSessionHandoff {
        session_bound: false,
    };

    let error = run_desktop_local_service_bootstrap(
        &plan,
        &mut runtime,
        &mut shell,
        &mut probe,
        &mut handoff,
        10,
    )
    .expect_err("session failure blocks bootstrap");

    assert!(matches!(
        error,
        DesktopLocalServiceBootstrapError::SessionHandoffFailed
    ));
    assert_eq!(
        runtime.snapshot().last_failure,
        Some(NativeProcessRuntimeFailureKind::SessionHandoffFailed)
    );
    assert!(shell.recovery_bootstrap_for_web().is_some());
}

#[test]
fn desktop_node_role_payload_maps_native_endpoint() {
    let plan = plan();
    let outcome = node_role_probe_outcome_from_json(
        &plan,
        &json!({
            "role": "native-main",
            "native_service": {
                "state": "session_pending",
                "endpoint": {
                    "http_base": "http://127.0.0.1:39101",
                    "ws_base": "ws://127.0.0.1:39101",
                    "node_role": "native-main",
                    "session_bound": false
                }
            }
        }),
    )
    .expect("node role payload");

    assert!(outcome.probe.is_healthy());
    assert_eq!(outcome.endpoint.node_role, "native-main");
    assert!(!outcome.endpoint.session_bound);
}

#[test]
fn desktop_node_role_payload_requires_top_level_role() {
    let plan = plan();
    let error = node_role_probe_outcome_from_json(
        &plan,
        &json!({
            "native_service": {
                "state": "session_pending",
                "endpoint": {
                    "http_base": "http://127.0.0.1:39101",
                    "ws_base": "ws://127.0.0.1:39101",
                    "node_role": "native-main",
                    "session_bound": false
                }
            }
        }),
    )
    .expect_err("missing top-level role fails closed");

    assert!(matches!(
        error,
        DesktopLocalServiceBootstrapError::InvalidNodeRolePayload
    ));
}

#[test]
fn desktop_node_role_payload_rejects_empty_fallback_role() {
    let plan = plan();
    let error = node_role_probe_outcome_from_json(&plan, &json!({"role": "  "}))
        .expect_err("empty fallback role fails closed");

    assert!(matches!(
        error,
        DesktopLocalServiceBootstrapError::InvalidNodeRolePayload
    ));
}

#[test]
fn desktop_node_role_payload_rejects_empty_endpoint_role() {
    let plan = plan();
    for node_role in ["", "  "] {
        let error = node_role_probe_outcome_from_json(
            &plan,
            &json!({
                "role": "native-main",
                "native_service": {
                    "state": "session_pending",
                    "endpoint": {
                        "http_base": "http://127.0.0.1:39101",
                        "ws_base": "ws://127.0.0.1:39101",
                        "node_role": node_role,
                        "session_bound": false
                    }
                }
            }),
        )
        .expect_err("empty endpoint role fails closed");

        assert!(matches!(
            error,
            DesktopLocalServiceBootstrapError::InvalidNodeRolePayload
        ));
    }
}

#[test]
fn desktop_auth_status_controls_session_material() {
    assert!(session_material_from_auth_status_json(&json!({"authenticated": true})).is_ok());
    assert!(matches!(
        session_material_from_auth_status_json(&json!({"authenticated": false})),
        Err(DesktopLocalServiceBootstrapError::SessionHandoffFailed)
    ));
}
