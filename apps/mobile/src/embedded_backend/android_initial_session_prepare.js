// plan_ref: 11_ui_design/03_mobile#mobile-native-shell-modes
((root, installId, initializeBootstrap, initialBootstrapStatus, failureCapabilities) => {
  const key = "__DEVE_NATIVE_SESSION_INSTALLED__";
  const phaseKey = "__DEVE_NATIVE_SESSION_PREPARE_PHASE__";
  const setPhase = (phase) => {
    root[phaseKey] = phase;
  };
  const fail = () => {
    root.__DEVE_NATIVE_BOOTSTRAP = {
      service_state: "session_invalid",
      platform_lifecycle_authority: "native",
      capabilities: failureCapabilities,
    };
    root.dispatchEvent(new root.Event("deve-native-service-error"));
  };

  const admissionRetryDelayMs = 25;
  const admissionMaxAttempts = 200;
  const bridgeAdmissionTimeoutMs = 5000;
  const bridgeReadyCommand =
    "plugin:deve-native-backend-commands|native_backend_webview_session_bridge_ready";
  const prepareCommand =
    "plugin:deve-native-backend-commands|native_backend_prepare_webview_session";
  let admissionAttempts = 0;
  let terminal = false;
  let inFlight = false;
  let prepareStarted = false;
  let bootstrapStatus = initialBootstrapStatus;
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
    setPhase("failed");
    fail();
  };
  const retryReadiness = () => {
    if (terminal) return;
    inFlight = false;
    admissionAttempts += 1;
    if (admissionAttempts >= admissionMaxAttempts) {
      failOnce();
      return;
    }
    root.setTimeout(attempt, admissionRetryDelayMs);
  };
  const admitBootstrapStorage = () => {
    if (root.__DEVE_NATIVE_SESSION_STORAGE_READY === true) return true;
    setPhase("bootstrap-storage");
    let status = bootstrapStatus;
    bootstrapStatus = undefined;
    if (status === undefined) {
      try {
        status = initializeBootstrap();
      } catch (_error) {
        failOnce();
        return false;
      }
    }
    if (status === "ready" && root.__DEVE_NATIVE_SESSION_STORAGE_READY === true) return true;
    if (status !== "storage_unavailable") {
      terminal = true;
      retireAdmissionTimer();
      setPhase("failed");
    }
    return false;
  };
  const attempt = () => {
    if (terminal || inFlight) return undefined;
    if (!admitBootstrapStorage()) {
      if (!terminal) retryReadiness();
      return undefined;
    }
    let installed;
    try {
      installed = root.sessionStorage.getItem(key) === installId;
    } catch (_error) {
      failOnce();
      return undefined;
    }
    if (installed) {
      terminal = true;
      retireAdmissionTimer();
      setPhase("installed");
      return undefined;
    }
    setPhase("bridge-readiness");
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
        setPhase("native-prepare");
        return invoke(prepareCommand);
      })
      .then(() => {
        if (!prepareStarted || terminal) return;
        root.sessionStorage.setItem(key, installId);
        if (root.sessionStorage.getItem(key) !== installId) {
          throw new Error("native session install marker unavailable");
        }
        terminal = true;
        setPhase("reload-pending");
        root.location.reload();
      })
      .catch(() => {
        if (prepareStarted) failOnce();
        else retryReadiness();
      });
  };
  setPhase("bootstrap-storage");
  root.setTimeout(attempt, 0);
  admissionTimer = root.setTimeout(failOnce, bridgeAdmissionTimeoutMs);
})
