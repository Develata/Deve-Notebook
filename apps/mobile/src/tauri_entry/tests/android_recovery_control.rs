//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes

#[test]
fn android_backend_recovery_control_is_restored_across_activity_and_webview_lifecycle() {
    let activity = include_str!(
        "../../../gen/android/app/src/main/java/dev/deve/notebook/mobile/MainActivity.kt"
    );
    let control = include_str!(
        "../../../gen/android/app/src/main/java/dev/deve/notebook/mobile/UseLocalBackendControl.kt"
    );

    assert!(activity.contains("UseLocalBackendControl(this) { requestUseLocalBackend() }"));
    assert!(activity.contains("override fun onResume()"));
    assert!(activity.contains("useLocalBackendControl.attach(webView)"));
    assert!(activity.contains("useLocalBackendControl.restoreIfDesired()"));
    assert!(activity.contains("useLocalBackendControl.detach()"));
    assert!(control.contains("var desired = false"));
    assert!(control.contains("desired = true"));
    assert!(control.contains("source.post"));
    assert!(control.contains("generation == expectedGeneration"));
    assert!(control.contains("webView === source"));
    assert!(control.contains("source.isAttachedToWindow"));
    assert!(control.contains("parent !== root"));
    assert!(control.contains("existing.bringToFront()"));
    assert!(control.contains("control.bringToFront()"));

    let remove = &control[control
        .find("fun remove(): Boolean")
        .expect("remove recovery control")..];
    let retire_desired = remove
        .find("desired = false")
        .expect("desired state retirement");
    let retire_generation = remove
        .find("generation += 1")
        .expect("generation retirement");
    let remove_view = remove
        .find("removeView(current)")
        .expect("native view retirement");
    assert!(retire_desired < retire_generation);
    assert!(retire_generation < remove_view);
}
