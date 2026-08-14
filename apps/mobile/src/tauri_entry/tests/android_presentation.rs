//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 11_ui_design/03_mobile#mobile-current-native-boundary

#[test]
fn android_system_gesture_insets_are_generation_bound_presentation_only_hints() {
    let activity = include_str!(
        "../../../gen/android/app/src/main/java/dev/deve/notebook/mobile/MainActivity.kt"
    );
    let presentation = include_str!(
        "../../../gen/android/app/src/main/java/dev/deve/notebook/mobile/NativePresentationDispatcher.kt"
    );
    let light_theme = include_str!("../../../gen/android/app/src/main/res/values/themes.xml");
    let dark_theme = include_str!("../../../gen/android/app/src/main/res/values-night/themes.xml");

    assert!(activity.contains("nativePresentationDispatcher.attach(webView)"));
    assert!(activity.contains("nativePresentationDispatcher.onWindowFocusChanged(hasFocus)"));
    assert!(activity.contains("nativePresentationDispatcher.detach()"));
    assert!(activity.contains("SystemBarStyle.auto(Color.TRANSPARENT, Color.TRANSPARENT)"));
    assert!(activity.contains("window.isNavigationBarContrastEnforced = false"));
    for theme in [light_theme, dark_theme] {
        assert!(theme.contains("android:statusBarColor\">@android:color/transparent"));
        assert!(theme.contains("android:navigationBarColor\">@android:color/transparent"));
        assert!(theme.contains("android:enforceStatusBarContrast"));
        assert!(theme.contains("android:enforceNavigationBarContrast"));
    }
    assert!(light_theme.contains("android:windowLightNavigationBar\">true"));
    assert!(dark_theme.contains("android:windowLightNavigationBar\">false"));
    assert!(presentation.contains("WindowInsetsCompat.Type.systemGestures()"));
    assert!(presentation.contains("activity.window.decorView"));
    assert!(presentation.contains("deve-native-presentation-change"));
    assert!(presentation.contains("__DEVE_ANDROID_PRESENTATION__"));
    assert!(presentation.contains("__DEVE_ANDROID_PRESENTATION_PENDING__"));
    assert!(presentation.contains("WebViewCompat.addDocumentStartJavaScript("));
    assert!(presentation.contains("WebViewCompat.addWebMessageListener("));
    assert!(presentation.contains("WebViewCompat.removeWebMessageListener("));
    assert!(presentation.contains("WebViewFeature.DOCUMENT_START_SCRIPT"));
    assert!(presentation.contains("WebViewFeature.WEB_MESSAGE_LISTENER"));
    assert!(presentation.contains("isMainFrame && view === webView"));
    assert!(presentation.contains("message.data == DOCUMENT_MESSAGE"));
    assert!(presentation.contains("kind: \"system-gesture-insets\""));
    assert!(presentation.contains("kind: \"system-gesture-insets-pending\""));
    assert!(presentation.contains("epoch: $epoch"));
    assert!(presentation.contains("widthPx: ${geometry.widthPx}"));
    assert!(presentation.contains("heightPx: ${geometry.heightPx}"));
    assert!(presentation.contains("leftPx: ${geometry.leftPx}"));
    assert!(presentation.contains("rightPx: ${geometry.rightPx}"));
    assert!(presentation.contains("safeTopPx: ${geometry.safeTopPx}"));
    assert!(presentation.contains("safeBottomPx: ${geometry.safeBottomPx}"));
    assert!(presentation.contains("imeVisible: ${geometry.imeVisible}"));
    assert!(presentation.contains("imeBottomPx: ${geometry.imeBottomPx}"));
    assert!(presentation.contains("webViewGeneration"));
    assert!(presentation.contains("publishEpoch"));
    assert!(presentation.contains("isCurrent(source, generation, epoch)"));
    assert!(presentation.contains("ViewCompat.setOnApplyWindowInsetsListener(observer)"));
    assert!(presentation.contains("ViewCompat.setOnApplyWindowInsetsListener(source)"));
    assert_eq!(
        presentation
            .matches("ViewCompat.setOnApplyWindowInsetsListener(source)")
            .count(),
        1
    );
    assert!(presentation.contains("ViewCompat.requestApplyInsets(source)"));
    assert!(presentation.contains("WindowInsetsCompat.Type.ime()"));
    assert!(presentation.contains("WindowInsetsCompat.Type.systemBars()"));
    assert!(presentation.contains("WindowInsetsCompat.Type.displayCutout()"));
    assert!(presentation.contains("PresentationGeometryRead.ImeOverlayOrUnavailable"));
    assert!(presentation.contains("android_webview_ime_insets_applied"));
    assert!(presentation.contains("android_webview_ime_overlay_or_unavailable"));
    assert!(
        presentation
            .contains("source?.let { ViewCompat.setOnApplyWindowInsetsListener(it, null) }")
    );
    assert!(!presentation.contains("WindowInsetsCompat.CONSUMED"));
    assert!(!presentation.contains("setPadding("));
    assert!(presentation.contains("root.addView(observer, FrameLayout.LayoutParams(0, 0))"));
    assert!(presentation.contains("listenerSeen: false"));
    assert!(presentation.contains("android_system_gesture_insets_ready"));
    assert!(presentation.contains("android_system_gesture_insets_unavailable"));
    assert!(!presentation.contains("DOCUMENT_PROBE_INTERVAL_MS"));
    assert!(!presentation.contains("cookie"));
    assert!(!presentation.contains("session"));
    assert!(!presentation.contains("endpoint"));

    let generated_source_root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("gen/android/app/src/main/java");
    let mut listener_call_owners = Vec::new();
    let mut pending_directories = vec![generated_source_root];
    while let Some(directory) = pending_directories.pop() {
        for entry in std::fs::read_dir(&directory).expect("generated Android source directory") {
            let path = entry.expect("generated Android source entry").path();
            if path.is_dir() {
                pending_directories.push(path);
                continue;
            }
            if !matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("kt" | "java")
            ) {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("generated Android source text");
            let calls = source
                .matches("ViewCompat.setOnApplyWindowInsetsListener(")
                .count();
            listener_call_owners.extend(std::iter::repeat_n(path, calls));
        }
    }
    assert_eq!(listener_call_owners.len(), 4);
    assert!(listener_call_owners.iter().all(|path| {
        path.file_name().and_then(|value| value.to_str()) == Some("NativePresentationDispatcher.kt")
    }));
}

#[test]
fn android_webview_input_focus_and_trusted_editor_tap_are_generation_owned() {
    let activity = include_str!(
        "../../../gen/android/app/src/main/java/dev/deve/notebook/mobile/MainActivity.kt"
    );
    let manifest = include_str!("../../../gen/android/app/src/main/AndroidManifest.xml");
    let coordinator = include_str!(
        "../../../gen/android/app/src/main/java/dev/deve/notebook/mobile/WebViewInputCoordinator.kt"
    );

    assert!(activity.contains("webViewInputCoordinator.attach(webView)"));
    assert!(activity.contains("webViewInputCoordinator.onWindowFocusChanged(hasFocus)"));
    assert!(activity.contains("webViewInputCoordinator.detach()"));
    assert!(!activity.contains("override fun dispatchTouchEvent(event: MotionEvent)"));
    assert!(manifest.contains("android:name=\".MainActivity\""));
    assert!(manifest.contains("android:windowSoftInputMode=\"adjustResize\""));
    assert!(!manifest.contains("android:windowSoftInputMode=\"adjustUnspecified\""));
    assert!(!manifest.contains("android:windowSoftInputMode=\"adjustPan\""));
    assert!(!manifest.contains("android:windowSoftInputMode=\"adjustNothing\""));
    assert!(!coordinator.contains("setOnApplyWindowInsetsListener"));
    assert!(coordinator.contains("webView.setOnTouchListener { view, event ->"));
    assert!(coordinator.contains("if (view === this.webView) onWebViewTouchEvent(event)"));
    assert!(coordinator.contains("webView?.setOnTouchListener(null)"));
    assert!(coordinator.contains("source.isFocusableInTouchMode = true"));
    assert!(coordinator.contains("!source.hasFocus() && !source.requestFocus()"));
    assert!(coordinator.contains("android_webview_input_focus_unavailable"));
    assert!(coordinator.contains("MotionEvent.ACTION_DOWN -> beginTapCandidate(event)"));
    assert!(
        coordinator.contains("MotionEvent.ACTION_MOVE -> retainTapCandidateIfStationary(event)")
    );
    assert!(coordinator.contains("MotionEvent.ACTION_UP -> completeTapCandidate(event)"));
    assert!(coordinator.contains("MotionEvent.ACTION_CANCEL"));
    assert!(coordinator.contains("MotionEvent.ACTION_POINTER_DOWN"));
    assert!(coordinator.contains("ViewConfiguration.get(webView.context).scaledTouchSlop"));
    assert!(!coordinator.contains("private val touchSlop = ViewConfiguration.get(activity)"));
    assert!(coordinator.contains(
        "event.eventTime - candidate.downTime >= ViewConfiguration.getLongPressTimeout()"
    ));
    assert!(coordinator.contains("document.elementFromPoint($xCss, $yCss)"));
    assert!(coordinator.contains(".cm-content[contenteditable=\"true\"]"));
    assert!(coordinator.contains("document.activeElement === editor"));
    assert!(coordinator.contains("webViewGeneration != generation"));
    assert!(
        coordinator.contains(
            "!activity.hasWindowFocus() || !source.isAttachedToWindow || !source.isShown"
        )
    );
    assert!(coordinator.contains(".show(WindowInsetsCompat.Type.ime())"));
    assert!(coordinator.contains("android_webview_ime_show_failed"));
    assert!(!coordinator.contains("showSoftInput"));
}
