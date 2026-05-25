//! plan_ref:
//!   - 11_ui_design_02_desktop#desktop-process-adapter-decision

use deve_core::native_adapter::NativeProcessSpawnSpec;

use super::DesktopProcessRuntimeError;

pub(super) fn validate_desktop_service_command(
    spec: &NativeProcessSpawnSpec,
) -> Result<(), DesktopProcessRuntimeError> {
    spec.validate_contract()?;
    let executable_name = spec
        .executable
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !matches!(executable_name, "deve_cli" | "deve_cli.exe") {
        return Err(DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "executable must be deve_cli",
        });
    }
    if spec.argv.first().map(String::as_str) != Some("serve") {
        return Err(DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "first argv must be serve",
        });
    }
    if spec.argv.len() != 4
        || spec.argv.get(1).map(String::as_str) != Some("--native-loopback")
        || spec.argv.get(2).map(String::as_str) != Some("--port")
    {
        return Err(DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "argv must be exactly serve --native-loopback --port <port>",
        });
    }
    let Some(port) = spec.argv.get(3).and_then(|value| parse_nonzero_port(value)) else {
        return Err(DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "argv port must be a non-zero u16",
        });
    };
    if spec.bind_hints.http_port != Some(port) || spec.bind_hints.ws_port != Some(port) {
        return Err(DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "argv port must match loopback bind hints",
        });
    }
    Ok(())
}

fn parse_nonzero_port(value: &str) -> Option<u16> {
    value.parse::<u16>().ok().filter(|port| *port != 0)
}
