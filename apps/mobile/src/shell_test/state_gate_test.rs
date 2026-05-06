use super::support::bound_shell;
use crate::{MobileServiceState, MobileShellError};
use deve_core::native_adapter::{
    NativeProcessAdapterState, NativeRuntimeReadiness, NativeServiceRestarting,
    NativeServiceSupervisorState,
};

#[test]
fn mobile_shell_offline_and_session_invalid_block_bootstrap() {
    let mut offline = bound_shell();
    offline.mark_service_offline("service_dead", true);

    assert!(matches!(
        offline.bootstrap_for_web(),
        Err(MobileShellError::ServiceOffline { reason }) if reason == "service_dead"
    ));
    let offline_script = offline
        .recovery_bootstrap_for_web()
        .expect("recovery bootstrap")
        .script_tag()
        .expect("recovery script");
    assert!(offline_script.contains("\"service_state\":\"service_offline\""));
    assert!(!offline_script.contains("service_dead"));
    let offline_snapshot = offline.snapshot();
    assert_eq!(
        offline_snapshot.state,
        MobileServiceState::ServiceRestarting
    );
    assert_eq!(
        offline_snapshot.restarting,
        Some(NativeServiceRestarting { attempt: 0 })
    );
    assert_eq!(
        offline_snapshot.readiness,
        NativeRuntimeReadiness::default()
    );
    assert_eq!(
        offline_snapshot.supervisor.state,
        NativeServiceSupervisorState::Restarting
    );
    assert_eq!(
        offline_snapshot.supervisor.offline,
        offline_snapshot.offline
    );
    assert!(offline_snapshot.endpoint.is_none());
    assert!(offline_snapshot.process_adapter.endpoint.is_none());

    let mut invalid = bound_shell();
    invalid.invalidate_session();
    assert!(matches!(
        invalid.bootstrap_for_web(),
        Err(MobileShellError::SessionInvalid)
    ));
    assert!(!invalid.snapshot().endpoint.expect("endpoint").session_bound);
    assert_eq!(
        invalid
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "session_invalid"
    );
    assert_eq!(
        invalid.snapshot().process_adapter.state,
        NativeProcessAdapterState::ExistingEndpointBound
    );
}
