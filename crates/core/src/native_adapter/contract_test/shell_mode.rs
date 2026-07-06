use crate::native_adapter::{NativeRemoteTarget, NativeShellMode, validate_native_remote_target};

#[test]
fn native_shell_mode_local_backend_is_default_and_starts_local_backend() {
    let mode = NativeShellMode::local_backend_default();

    assert_eq!(mode, NativeShellMode::LocalBackend);
    assert!(mode.starts_local_backend());
}

#[test]
fn remote_browser_accepts_https_origin_only() {
    let mode = NativeShellMode::remote_browser("https://deve.example");
    let NativeShellMode::RemoteBrowser { target } = mode else {
        panic!("remote browser mode expected");
    };

    assert_eq!(validate_native_remote_target(&target), Ok(()));
    assert_eq!(
        validate_native_remote_target(&NativeRemoteTarget {
            https_origin: "https://deve.example:443".to_string(),
        }),
        Ok(())
    );
    assert_eq!(
        validate_native_remote_target(&NativeRemoteTarget {
            https_origin: "https://[::1]:8443".to_string(),
        }),
        Ok(())
    );

    for invalid in [
        "http://deve.example",
        "https://user@deve.example",
        "https://deve.example/",
        "https://deve.example/app",
        "https://deve.example?token=secret",
        "https://deve.example#fragment",
        "https://deve.example:0",
        "https://:443",
        "https://bad host",
        "https://deve.example\\app",
        "https://deve.example:443:evil",
        "https://[::1",
        "https://[]",
        " https://deve.example",
    ] {
        let target = NativeRemoteTarget {
            https_origin: invalid.to_string(),
        };
        assert!(
            validate_native_remote_target(&target).is_err(),
            "invalid remote target accepted: {invalid}"
        );
    }
}
