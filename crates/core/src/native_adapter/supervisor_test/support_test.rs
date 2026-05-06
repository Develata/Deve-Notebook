use super::super::{
    NativeEndpointReady, NativeProcessAdapter, NativeProcessAdapterSnapshot,
    NativeServiceHealthProbe,
};

pub(crate) fn service_probe() -> NativeServiceHealthProbe {
    NativeServiceHealthProbe {
        endpoint_reachable: true,
        node_role_readable: true,
    }
}

pub(crate) fn service_endpoint() -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
        session_bound: false,
    }
}

pub(crate) fn ready_process_snapshot() -> NativeProcessAdapterSnapshot {
    let mut process = NativeProcessAdapter::default();
    process
        .bind_existing_endpoint(service_endpoint())
        .expect("endpoint");
    process.bind_session(true).expect("session")
}
