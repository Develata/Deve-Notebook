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

  const bridgeRetryDelayMs = 25;
  const bridgeMaxAttempts = 200;
  let bridgeAttempts = 0;
  let terminal = false;
  let inFlight = false;
  const failOnce = () => {
    if (terminal) return;
    terminal = true;
    fail();
  };
  const attempt = () => {
    if (terminal || inFlight) return undefined;
    const invoke = root.__TAURI_INTERNALS__?.invoke;
    if (typeof invoke !== "function") {
      bridgeAttempts += 1;
      if (bridgeAttempts >= bridgeMaxAttempts) {
        failOnce();
        return undefined;
      }
      root.setTimeout(attempt, bridgeRetryDelayMs);
      return undefined;
    }
    inFlight = true;
    return Promise.resolve()
      .then(() => invoke(
        "plugin:deve-native-backend-commands|native_backend_prepare_webview_session",
      ))
      .then(() => {
        root.sessionStorage.setItem(key, installId);
        if (root.sessionStorage.getItem(key) !== installId) {
          throw new Error("native session install marker unavailable");
        }
        terminal = true;
        root.location.reload();
      })
      .catch(failOnce);
  };
  root.setTimeout(attempt, 0);
})
