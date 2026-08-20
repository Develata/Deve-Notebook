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
  const bridgeAdmissionTimeoutMs = 5000;
  const bridgeReadyCommand =
    "plugin:deve-native-backend-commands|native_backend_webview_session_bridge_ready";
  const prepareCommand =
    "plugin:deve-native-backend-commands|native_backend_prepare_webview_session";
  let bridgeAttempts = 0;
  let terminal = false;
  let inFlight = false;
  let prepareStarted = false;
  let admissionTimer;
  const retireAdmissionTimer = () => {
    if (admissionTimer === undefined) return;
    root.clearTimeout(admissionTimer);
    admissionTimer = undefined;
  };
  const failOnce = () => {
    if (terminal) return;
    terminal = true;
    retireAdmissionTimer();
    fail();
  };
  const retryReadiness = () => {
    if (terminal) return;
    inFlight = false;
    bridgeAttempts += 1;
    if (bridgeAttempts >= bridgeMaxAttempts) {
      failOnce();
      return;
    }
    root.setTimeout(attempt, bridgeRetryDelayMs);
  };
  const attempt = () => {
    if (terminal || inFlight) return undefined;
    const invoke = root.__TAURI_INTERNALS__?.invoke;
    if (typeof invoke !== "function") {
      retryReadiness();
      return undefined;
    }
    inFlight = true;
    return Promise.resolve()
      .then(() => invoke(bridgeReadyCommand))
      .then((ready) => {
        if (terminal) return undefined;
        if (ready !== true) {
          retryReadiness();
          return undefined;
        }
        retireAdmissionTimer();
        prepareStarted = true;
        return invoke(prepareCommand);
      })
      .then(() => {
        if (!prepareStarted || terminal) return;
        root.sessionStorage.setItem(key, installId);
        if (root.sessionStorage.getItem(key) !== installId) {
          throw new Error("native session install marker unavailable");
        }
        terminal = true;
        root.location.reload();
      })
      .catch(() => {
        if (prepareStarted) failOnce();
        else retryReadiness();
      });
  };
  root.setTimeout(attempt, 0);
  admissionTimer = root.setTimeout(failOnce, bridgeAdmissionTimeoutMs);
})
