use super::{
    NodeRoleResponse, is_db_lock_error, read_main_owner_hint, read_main_port_hint,
    trusted_main_port, trusted_owner_endpoint,
};
use crate::local_cli_proxy_contract::LocalCliOwnerHint;
use deve_core::ledger::LocalAuthorityError;
use deve_core::models::RepoId;

fn node_role(role: &str, ws_port: u16, main_port: u16) -> NodeRoleResponse {
    NodeRoleResponse {
        role: role.into(),
        ws_port,
        main_port,
        host_peer_id: Some("aaaaaaaaaaaa".into()),
        runtime_incarnation: Some(uuid::Uuid::from_u128(1)),
        environment: None,
    }
}

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
    assert!(err.to_string().contains("Invalid main port owner hint"));
}

#[test]
fn owner_hint_round_trip_requires_exact_process_identity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let host = dir.path().join(".host");
    std::fs::create_dir_all(&host).expect("host dir");
    let hint = LocalCliOwnerHint::new(3001, "aaaaaaaaaaaa".into(), uuid::Uuid::from_u128(1));
    std::fs::write(
        host.join("main_port"),
        serde_json::to_vec(&hint).expect("encode hint"),
    )
    .expect("write hint");
    assert_eq!(
        read_main_owner_hint(dir.path()).expect("hint"),
        Some(hint.clone())
    );
    assert!(trusted_owner_endpoint(
        &hint,
        &node_role("main", 3001, 3001)
    ));

    let mut wrong_runtime = node_role("main", 3001, 3001);
    wrong_runtime.runtime_incarnation = Some(uuid::Uuid::from_u128(2));
    assert!(!trusted_owner_endpoint(&hint, &wrong_runtime));
    let mut wrong_host = node_role("main", 3001, 3001);
    wrong_host.host_peer_id = Some("bbbbbbbbbbbb".into());
    assert!(!trusted_owner_endpoint(&hint, &wrong_host));
}

#[test]
fn trusted_main_port_accepts_main_and_proxy_node_roles() {
    assert_eq!(
        trusted_main_port(&node_role("main", 3001, 3001), 3001,),
        Some(3001)
    );
    assert_eq!(
        trusted_main_port(&node_role("native-main", 3002, 3002), 3002,),
        Some(3002)
    );
    assert_eq!(
        trusted_main_port(&node_role("proxy", 3002, 3001), 3002,),
        Some(3001)
    );
}

#[test]
fn trusted_main_port_rejects_foreign_or_mismatched_node_role_payloads() {
    for role in [
        node_role("unknown", 3001, 3001),
        node_role("main", 3002, 3001),
        node_role("proxy", 3002, 0),
        node_role("proxy", 3001, 3001),
    ] {
        assert_eq!(trusted_main_port(&role, 3001), None);
    }
}

#[test]
fn typed_local_authority_busy_selects_the_authenticated_proxy_path() {
    let repo_id = RepoId::new_v4();
    let error = anyhow::Error::new(LocalAuthorityError::Busy(repo_id))
        .context("offline composition failed");
    assert!(is_db_lock_error(&error));
}
