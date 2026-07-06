use crate::native_adapter::{
    NativeBackendMode, NativeBackendPreference, NativeBackendValidationResult, NativeShellMode,
    native_shell_mode_for_backend_preference, validate_native_backend_preference,
};

// SET-007A: native backend preference is host-local shell config. The preference
// contract below (defaults to local, canonicalizes local by dropping any remote_url,
// remote requires a valid bare https origin) is the automated half of the case; the
// browser-section-unavailable and route/storage-absent checks are the manual Chrome
// walkthrough.
#[test]
fn native_backend_preference_defaults_to_local_backend() {
    let preference = NativeBackendPreference::default();

    assert_eq!(preference, NativeBackendPreference::local());
    assert_eq!(
        native_shell_mode_for_backend_preference(&preference),
        Ok(NativeShellMode::LocalBackend)
    );
    assert_eq!(validate_native_backend_preference(&preference), Ok(()));
}

#[test]
fn native_backend_preference_canonicalizes_local_without_remote_url() {
    let preference = NativeBackendPreference {
        mode: NativeBackendMode::Local,
        remote_url: Some("https://deve.example".into()),
    };

    assert_eq!(preference.canonicalized(), NativeBackendPreference::local());
}

#[test]
fn native_backend_preference_remote_maps_to_remote_browser() {
    let preference = NativeBackendPreference::remote("https://deve.example:8443");

    let shell_mode =
        native_shell_mode_for_backend_preference(&preference).expect("remote preference");
    let NativeShellMode::RemoteBrowser { target } = shell_mode else {
        panic!("remote preference must map to RemoteBrowser");
    };

    assert_eq!(target.https_origin, "https://deve.example:8443");
    assert_eq!(validate_native_backend_preference(&preference), Ok(()));
}

#[test]
fn native_backend_preference_remote_requires_valid_https_origin() {
    for preference in [
        NativeBackendPreference {
            mode: NativeBackendMode::Remote,
            remote_url: None,
        },
        NativeBackendPreference {
            mode: NativeBackendMode::Remote,
            remote_url: Some(String::new()),
        },
        NativeBackendPreference::remote("http://deve.example"),
        NativeBackendPreference::remote("https://deve.example/"),
        NativeBackendPreference::remote("https://deve.example/app"),
    ] {
        assert!(
            validate_native_backend_preference(&preference).is_err(),
            "invalid preference accepted: {preference:?}"
        );
    }
}

#[test]
fn native_backend_validation_result_serializes_without_secret_fields() {
    let ok = NativeBackendValidationResult::success("https://deve.example", "full-peer");
    let json = serde_json::to_string(&ok).expect("validation result json");

    assert!(json.contains("https://deve.example"));
    assert!(json.contains("full-peer"));
    assert!(!json.to_ascii_lowercase().contains("token"));
    assert!(!json.to_ascii_lowercase().contains("secret"));
}
