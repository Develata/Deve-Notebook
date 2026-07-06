use crate::{
    DEVE_DESKTOP_LOCAL_SERVICE_ENV, DEVE_NATIVE_AUTHORITY_ENV, DesktopLocalServiceEntrypointError,
    DesktopLocalServiceEntrypointPolicy, desktop_local_service_entrypoint_policy_from_env,
    plan_desktop_local_service_entrypoint,
};

use super::{ENV_LOCK, EnvGuard, input};

#[test]
fn desktop_local_service_entrypoint_is_local_backend_by_default() {
    let plan = plan_desktop_local_service_entrypoint(
        DesktopLocalServiceEntrypointPolicy::local_backend_default(),
        input(),
    )
    .expect("plan");

    assert!(plan.is_some());
}

#[test]
fn desktop_local_service_entrypoint_env_defaults_to_local_backend_and_allows_explicit_disable() {
    let _lock = ENV_LOCK.lock().expect("env lock");

    {
        let _guard = EnvGuard::set(&[
            (DEVE_NATIVE_AUTHORITY_ENV, None),
            (DEVE_DESKTOP_LOCAL_SERVICE_ENV, None),
        ]);
        assert_eq!(
            desktop_local_service_entrypoint_policy_from_env().expect("policy"),
            DesktopLocalServiceEntrypointPolicy::local_backend_default()
        );
    }

    {
        let _guard = EnvGuard::set(&[
            (DEVE_NATIVE_AUTHORITY_ENV, Some("0")),
            (DEVE_DESKTOP_LOCAL_SERVICE_ENV, None),
        ]);
        assert_eq!(
            desktop_local_service_entrypoint_policy_from_env().expect("policy"),
            DesktopLocalServiceEntrypointPolicy::local_backend_default()
        );
    }

    {
        let _guard = EnvGuard::set(&[
            (DEVE_NATIVE_AUTHORITY_ENV, None),
            (DEVE_DESKTOP_LOCAL_SERVICE_ENV, Some("0")),
        ]);
        assert_eq!(
            desktop_local_service_entrypoint_policy_from_env().expect("policy"),
            DesktopLocalServiceEntrypointPolicy::disabled()
        );
    }
}

#[test]
fn desktop_local_service_entrypoint_env_rejects_invalid_opt_in_value() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::set(&[
        (DEVE_NATIVE_AUTHORITY_ENV, Some("maybe")),
        (DEVE_DESKTOP_LOCAL_SERVICE_ENV, Some("1")),
    ]);

    let error =
        desktop_local_service_entrypoint_policy_from_env().expect_err("invalid env fails closed");

    assert!(matches!(
        error,
        DesktopLocalServiceEntrypointError::InvalidOptInValue {
            env: DEVE_NATIVE_AUTHORITY_ENV,
            ..
        }
    ));
}
