use super::{NodeRoleResponse, read_main_port_hint, trusted_main_port};

#[test]
fn missing_main_port_hint_is_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let port = read_main_port_hint(dir.path()).expect("missing hint should be allowed");
    assert_eq!(port, None);
}

#[test]
fn invalid_main_port_hint_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let host = dir.path().join(".host");
    std::fs::create_dir_all(&host).expect("host dir");
    std::fs::write(host.join("main_port"), "not-a-port").expect("write hint");

    let err = read_main_port_hint(dir.path()).expect_err("invalid hint must fail closed");
    assert!(err.to_string().contains("Invalid main port hint"));
}

#[test]
fn trusted_main_port_accepts_main_and_proxy_node_roles() {
    assert_eq!(
        trusted_main_port(
            &NodeRoleResponse {
                role: "main".into(),
                ws_port: 3001,
                main_port: 3001,
            },
            3001,
        ),
        Some(3001)
    );
    assert_eq!(
        trusted_main_port(
            &NodeRoleResponse {
                role: "native-main".into(),
                ws_port: 3002,
                main_port: 3002,
            },
            3002,
        ),
        Some(3002)
    );
    assert_eq!(
        trusted_main_port(
            &NodeRoleResponse {
                role: "proxy".into(),
                ws_port: 3002,
                main_port: 3001,
            },
            3002,
        ),
        Some(3001)
    );
}

#[test]
fn trusted_main_port_rejects_foreign_or_mismatched_node_role_payloads() {
    for role in [
        NodeRoleResponse {
            role: "unknown".into(),
            ws_port: 3001,
            main_port: 3001,
        },
        NodeRoleResponse {
            role: "main".into(),
            ws_port: 3002,
            main_port: 3001,
        },
        NodeRoleResponse {
            role: "proxy".into(),
            ws_port: 3002,
            main_port: 0,
        },
        NodeRoleResponse {
            role: "proxy".into(),
            ws_port: 3001,
            main_port: 3001,
        },
    ] {
        assert_eq!(trusted_main_port(&role, 3001), None);
    }
}
