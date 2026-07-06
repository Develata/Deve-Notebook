use crate::native_adapter::{NativeProcessEnvBinding, NativeProcessRuntimeError};

use super::valid_spawn_spec;

#[test]
fn process_spawn_spec_rejects_empty_executable() {
    let mut spec = valid_spawn_spec();
    spec.executable = std::path::PathBuf::new();

    assert_eq!(
        spec.validate_contract(),
        Err(NativeProcessRuntimeError::EmptyPath {
            field: "executable"
        })
    );
}

#[test]
fn process_spawn_spec_rejects_relative_executable_without_resolver() {
    let mut spec = valid_spawn_spec();
    spec.executable = "deve_cli".into();

    assert_eq!(
        spec.validate_contract(),
        Err(NativeProcessRuntimeError::RelativePathForbidden {
            field: "executable"
        })
    );
}

#[test]
fn process_spawn_spec_rejects_unknown_environment_variable() {
    let mut spec = valid_spawn_spec();
    spec.env.push(NativeProcessEnvBinding {
        key: "AUTH_SECRET".to_string(),
        value: "must-not-be-forwarded".to_string(),
    });

    assert_eq!(
        spec.validate_contract(),
        Err(
            NativeProcessRuntimeError::EnvironmentVariableNotAllowlisted {
                key: "AUTH_SECRET".to_string()
            }
        )
    );
}

#[test]
fn process_spawn_spec_rejects_non_loopback_bind_hints() {
    let mut spec = valid_spawn_spec();
    spec.bind_hints.http_host = "0.0.0.0".to_string();

    assert_eq!(
        spec.validate_contract(),
        Err(NativeProcessRuntimeError::NonLoopbackBindHost { field: "http_host" })
    );
}
