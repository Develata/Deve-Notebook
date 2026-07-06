use std::path::PathBuf;

use crate::{
    DEVE_DESKTOP_DATA_DIR_ENV, DEVE_DESKTOP_LOCAL_SERVICE_ENV, DEVE_NATIVE_AUTHORITY_ENV,
    DesktopLocalServiceEntrypointError, DesktopLocalServiceEntrypointPolicy,
    plan_desktop_local_service_entrypoint,
    plan_desktop_local_service_entrypoint_for_current_process,
};

use super::{ENV_LOCK, EnvGuard, input};

#[test]
fn desktop_local_service_entrypoint_uses_app_private_data_root_override() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let data_root = std::env::temp_dir().join(format!("deve-desktop-data-root-test-{suffix}"));
    let data_root_str = data_root.to_string_lossy().to_string();
    let _guard = EnvGuard::set(&[
        (DEVE_NATIVE_AUTHORITY_ENV, None),
        (DEVE_DESKTOP_LOCAL_SERVICE_ENV, None),
        (DEVE_DESKTOP_DATA_DIR_ENV, Some(data_root_str.as_str())),
    ]);

    let plan = plan_desktop_local_service_entrypoint_for_current_process(
        DesktopLocalServiceEntrypointPolicy::local_backend_default(),
    )
    .expect("plan")
    .expect("enabled plan");

    assert_eq!(plan.spawn_spec.cwd, data_root);
    assert_eq!(
        plan.spawn_spec.ledger_path,
        plan.spawn_spec.cwd.join("ledger")
    );
}

#[test]
fn desktop_local_service_entrypoint_rejects_relative_data_root() {
    let mut request = input();
    request.data_root = PathBuf::from("relative-data");

    let error = plan_desktop_local_service_entrypoint(
        DesktopLocalServiceEntrypointPolicy::opt_in_enabled(),
        request,
    )
    .expect_err("relative paths fail closed");

    assert!(matches!(
        error,
        DesktopLocalServiceEntrypointError::InvalidSpawnSpec(_)
    ));
}
