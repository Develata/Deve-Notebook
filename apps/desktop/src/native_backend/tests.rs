use super::*;

#[test]
fn desktop_native_backend_preference_roundtrips_host_local_json() {
    let root = unique_temp_root("roundtrip");
    let preference = NativeBackendPreference::remote("https://deve.example");

    save_desktop_native_backend_preference(&root, &preference).expect("save preference");
    let loaded = load_desktop_native_backend_preference(&root).expect("load preference");

    assert_eq!(loaded, preference);
    let contents = std::fs::read_to_string(desktop_native_backend_config_path(&root))
        .expect("config contents");
    assert!(!contents.to_ascii_lowercase().contains("token"));
    assert!(!contents.to_ascii_lowercase().contains("scope_nonce"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn desktop_native_backend_local_preference_drops_remote_url() {
    let root = unique_temp_root("local");
    let preference = NativeBackendPreference {
        mode: NativeBackendMode::Local,
        remote_url: Some("https://deve.example".into()),
    };

    save_desktop_native_backend_preference(&root, &preference).expect("save preference");
    let loaded = load_desktop_native_backend_preference(&root).expect("load preference");

    assert_eq!(loaded, NativeBackendPreference::local());
    let contents = std::fs::read_to_string(desktop_native_backend_config_path(&root))
        .expect("config contents");
    assert!(!contents.contains("remote_url"));
    assert!(!contents.contains("https://deve.example"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn desktop_native_backend_concurrent_saves_publish_valid_json() {
    let root = unique_temp_root("concurrent");
    let workers = (0..16)
        .map(|index| {
            let root = root.clone();
            std::thread::spawn(move || {
                let preference =
                    NativeBackendPreference::remote(format!("https://deve-{index}.example"));
                save_desktop_native_backend_preference(&root, &preference)
            })
        })
        .collect::<Vec<_>>();

    for worker in workers {
        worker
            .join()
            .expect("concurrent save worker")
            .expect("concurrent save");
    }
    let loaded = load_desktop_native_backend_preference(&root).expect("load final preference");
    assert!(matches!(loaded.mode, NativeBackendMode::Remote));
    assert!(loaded.remote_url.is_some());
    assert!(desktop_native_backend_config_path(&root).is_file());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn desktop_native_backend_post_publish_failure_seals_until_reload() {
    let root = unique_temp_root("post-publish");
    std::fs::create_dir_all(&root).expect("create data root");
    std::fs::write(root.join(".native-backend-fail-after-replace"), b"fail")
        .expect("install failure marker");
    let state = DesktopNativeBackendState::from_data_root(Ok::<_, &str>(root.clone()));
    let preference = NativeBackendPreference::remote("https://deve.example");

    assert!(matches!(
        state.save_preference(preference.clone()),
        Err(DesktopNativeBackendError::WriteFailed(_))
    ));
    assert!(matches!(
        state.preference(),
        Err(DesktopNativeBackendError::DurabilityUncertain)
    ));

    std::fs::remove_file(root.join(".native-backend-fail-after-replace"))
        .expect("remove failure marker");
    let reloaded = DesktopNativeBackendState::from_data_root(Ok::<_, &str>(root.clone()));
    assert_eq!(
        reloaded.preference().expect("reloaded preference"),
        preference
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn desktop_native_remote_origin_normalization_keeps_https_origin_only() {
    assert_eq!(
        normalized_native_remote_origin("https://deve.example/").expect("origin"),
        "https://deve.example"
    );
    assert!(normalized_native_remote_origin("https://deve.example//").is_err());
    assert!(normalized_native_remote_origin("http://deve.example").is_err());
    assert!(normalized_native_remote_origin("https://deve.example/app").is_err());
}

#[test]
fn desktop_native_remote_probe_error_reports_redirect_boundary() {
    assert_eq!(
        DesktopNativeBackendError::ProbeRedirected.to_string(),
        "desktop remote backend probe redirected away from requested origin"
    );
}

#[test]
fn desktop_native_remote_probe_rejects_cross_origin_response_url() {
    let same_origin =
        reqwest::Url::parse("https://deve.example/api/node/role").expect("same origin url");
    let other_origin =
        reqwest::Url::parse("https://other.example/api/node/role").expect("other origin url");

    ensure_probe_response_origin("https://deve.example", &same_origin).expect("same origin");
    assert!(matches!(
        ensure_probe_response_origin("https://deve.example", &other_origin),
        Err(DesktopNativeBackendError::ProbeRedirected)
    ));
}

fn unique_temp_root(name: &str) -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let root = std::env::temp_dir().join(format!("deve-desktop-native-backend-{name}-{}", suffix));
    let _ = std::fs::remove_dir_all(&root);
    root
}
