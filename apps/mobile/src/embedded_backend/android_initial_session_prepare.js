// plan_ref: 11_ui_design/03_mobile#mobile-native-shell-modes
((root, installId) => {
  const key = "__DEVE_NATIVE_SESSION_INSTALLED__";
  const current = root.__DEVE_NATIVE_BOOTSTRAP;
  const fail = () => {
    root.__DEVE_NATIVE_BOOTSTRAP = {
      service_state: "session_invalid",
      platform_lifecycle_authority: "native",
      capabilities: current?.capabilities,
    };
    root.dispatchEvent(new root.Event("deve-native-service-error"));
  };
  let installed = false;
  try {
    if (root.__DEVE_NATIVE_SESSION_STORAGE_READY !== true) {
      if (current?.service_state !== "session_invalid") fail();
      return;
    }
    installed = root.sessionStorage.getItem(key) === installId;
  } catch (_error) {
    fail();
    return;
  }
  if (installed) return;

  // A newly created Android WebView can execute document-start scripts before
  // Tauri has finished registering that WebView on the native side. Yield one
  // browser task so the first IPC request cannot race that registration.
  root.setTimeout(() => Promise.resolve()
    .then(() => root.__TAURI_INTERNALS__.invoke(
      "plugin:deve-native-backend-commands|native_backend_prepare_webview_session",
    ))
    .then(() => {
      root.sessionStorage.setItem(key, installId);
      if (root.sessionStorage.getItem(key) !== installId) {
        throw new Error("native session install marker unavailable");
      }
      root.location.reload();
    })
    .catch(fail), 0);
})
