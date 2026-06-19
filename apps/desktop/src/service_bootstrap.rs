//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-service-supervisor-contract
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision

mod probe;

use deve_core::native_adapter::{
    NativeAdapterError, NativeEndpointReady, NativeProcessRuntimeSnapshot,
    NativeServiceFailureKind, NativeServiceHealthProbe, validate_native_endpoint_bases,
};
use serde_json::Value;
use thiserror::Error;

use crate::{
    DesktopBootstrap, DesktopLocalServiceEntrypointPlan, DesktopLocalServiceRuntime,
    DesktopProcessLauncher, DesktopProcessRuntimeError, DesktopSessionMaterial, DesktopShell,
    DesktopShellError,
};

pub use probe::DesktopLoopbackHttpProbe;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopLocalServiceProbeOutcome {
    pub endpoint: NativeEndpointReady,
    pub probe: NativeServiceHealthProbe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopLocalServiceBootstrapResult {
    pub bootstrap: DesktopBootstrap,
    pub bootstrap_script: String,
    pub session_material: DesktopSessionMaterial,
    pub runtime_snapshot: NativeProcessRuntimeSnapshot,
}

pub trait DesktopLocalServiceProbe {
    fn probe_node_role(
        &mut self,
        plan: &DesktopLocalServiceEntrypointPlan,
    ) -> Result<DesktopLocalServiceProbeOutcome, DesktopLocalServiceBootstrapError>;
}

pub trait DesktopLocalServiceSessionHandoff {
    fn bind_session(
        &mut self,
        plan: &DesktopLocalServiceEntrypointPlan,
        endpoint: &NativeEndpointReady,
    ) -> Result<DesktopSessionMaterial, DesktopLocalServiceBootstrapError>;
}

#[derive(Debug, Error)]
pub enum DesktopLocalServiceBootstrapError {
    #[error(transparent)]
    Runtime(#[from] DesktopProcessRuntimeError),
    #[error(transparent)]
    Shell(#[from] DesktopShellError),
    #[error("desktop local service health probe failed")]
    HealthProbeFailed,
    #[error("desktop local service session handoff failed")]
    SessionHandoffFailed,
    #[error("desktop local service probe URL is invalid")]
    InvalidProbeUrl,
    #[error("desktop local service endpoint is invalid")]
    InvalidEndpoint(#[from] NativeAdapterError),
    #[error("desktop local service probe HTTP status is not successful: {status}")]
    ProbeHttpStatus { status: u16 },
    #[error("desktop local service probe response is too large")]
    ProbeResponseTooLarge,
    #[error("desktop local service probe response is invalid")]
    ProbeInvalidResponse,
    #[error("desktop local service probe IO failed")]
    ProbeIo(#[source] std::io::Error),
    #[error("desktop native session bootstrap secret is missing")]
    MissingNativeSessionBootstrapSecret,
    #[error("desktop local service node-role payload is invalid")]
    InvalidNodeRolePayload,
    #[error("desktop native session cookie is invalid")]
    NativeSessionCookieInvalid,
}

pub fn run_desktop_local_service_bootstrap<L, P, H>(
    plan: &DesktopLocalServiceEntrypointPlan,
    runtime: &mut DesktopLocalServiceRuntime<L>,
    shell: &mut DesktopShell,
    probe: &mut P,
    handoff: &mut H,
    timestamp_unix_ms: i64,
) -> Result<DesktopLocalServiceBootstrapResult, DesktopLocalServiceBootstrapError>
where
    L: DesktopProcessLauncher,
    P: DesktopLocalServiceProbe,
    H: DesktopLocalServiceSessionHandoff,
{
    runtime.start(&plan.spawn_spec, timestamp_unix_ms)?;
    shell.start_service();

    let probe_outcome = match probe.probe_node_role(plan) {
        Ok(outcome) if outcome.probe.is_healthy() => outcome,
        Ok(outcome) => {
            runtime.record_endpoint_probe(
                outcome.endpoint,
                outcome.probe,
                timestamp_unix_ms.saturating_add(1),
            );
            shell.mark_supervisor_failure(
                NativeServiceFailureKind::HealthProbeFailed,
                "probe_failed",
            );
            return Err(DesktopLocalServiceBootstrapError::HealthProbeFailed);
        }
        Err(error) => {
            runtime.record_health_probe_failure(timestamp_unix_ms.saturating_add(1));
            shell.mark_supervisor_failure(
                NativeServiceFailureKind::HealthProbeFailed,
                "probe_failed",
            );
            return Err(error);
        }
    };

    let endpoint_snapshot = runtime.record_endpoint_probe(
        probe_outcome.endpoint.clone(),
        probe_outcome.probe,
        timestamp_unix_ms.saturating_add(1),
    );
    shell.bind_endpoint(probe_outcome.endpoint.clone())?;

    let session = match handoff.bind_session(plan, &probe_outcome.endpoint) {
        Ok(session) => session,
        Err(error) => {
            runtime.record_session_handoff(false, timestamp_unix_ms.saturating_add(2));
            shell.mark_supervisor_failure(
                NativeServiceFailureKind::SessionHandoffFailed,
                "session_not_bound",
            );
            return Err(error);
        }
    };
    let session_material = session.clone();
    shell.bind_session(session).map_err(|error| {
        runtime.record_session_handoff(false, timestamp_unix_ms.saturating_add(2));
        DesktopLocalServiceBootstrapError::Shell(error)
    })?;
    let runtime_snapshot =
        runtime.record_session_handoff(true, timestamp_unix_ms.saturating_add(2));
    let bootstrap = shell.bootstrap_for_web()?;
    let bootstrap_script = bootstrap.script_tag()?;

    debug_assert!(endpoint_snapshot.health_probe.is_healthy());
    Ok(DesktopLocalServiceBootstrapResult {
        bootstrap,
        bootstrap_script,
        session_material,
        runtime_snapshot,
    })
}

pub fn node_role_probe_outcome_from_json(
    plan: &DesktopLocalServiceEntrypointPlan,
    json: &Value,
) -> Result<DesktopLocalServiceProbeOutcome, DesktopLocalServiceBootstrapError> {
    let endpoint = match json.pointer("/native_service/endpoint") {
        Some(Value::Object(endpoint)) => endpoint_from_json(endpoint)?,
        _ => NativeEndpointReady {
            http_base: plan.http_base.clone(),
            ws_base: plan.ws_base.clone(),
            node_role: json
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("native-main")
                .to_string(),
            session_bound: false,
        },
    };
    validate_native_endpoint_bases(&endpoint)?;
    Ok(DesktopLocalServiceProbeOutcome {
        probe: NativeServiceHealthProbe {
            endpoint_reachable: true,
            node_role_readable: !endpoint.node_role.trim().is_empty(),
        },
        endpoint,
    })
}

pub fn session_material_from_auth_status_json(
    json: &Value,
) -> Result<DesktopSessionMaterial, DesktopLocalServiceBootstrapError> {
    if json
        .get("authenticated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        Ok(DesktopSessionMaterial::bound())
    } else {
        Err(DesktopLocalServiceBootstrapError::SessionHandoffFailed)
    }
}

fn endpoint_from_json(
    endpoint: &serde_json::Map<String, Value>,
) -> Result<NativeEndpointReady, DesktopLocalServiceBootstrapError> {
    Ok(NativeEndpointReady {
        http_base: string_field(endpoint, "http_base")?,
        ws_base: string_field(endpoint, "ws_base")?,
        node_role: string_field(endpoint, "node_role")?,
        session_bound: endpoint
            .get("session_bound")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn string_field(
    object: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<String, DesktopLocalServiceBootstrapError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(DesktopLocalServiceBootstrapError::InvalidNodeRolePayload)
}
