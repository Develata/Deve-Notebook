const WEBVIEW_INPUT_FOCUS_SETTLE_MS = 250;
const NATIVE_INPUT_PASSIVE_TIMEOUT_MS = 1_500;
const NATIVE_INPUT_REENTRY_TIMEOUT_MS = 30_000;
const ANDROID_APP_ID_PATTERN = /^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$/;
const ANDROID_COMPONENT_PATTERN =
  /(?:^|[\s{])([A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+)\/([A-Za-z0-9_.$]+)(?=[\s}]|$)/g;

function classifyExactPackageComponent(
  records,
  appId,
  containerPattern,
  matchedState,
  otherState,
) {
  const components = records.map((line) => {
    const containers = [...line.matchAll(containerPattern)];
    if (containers.length !== 1) return null;
    const matches = [...containers[0][1].matchAll(ANDROID_COMPONENT_PATTERN)];
    return matches.length === 1 ? `${matches[0][1]}/${matches[0][2]}` : null;
  });
  if (components.some((component) => component === null)) return "unavailable";
  const uniqueComponents = new Set(components);
  if (uniqueComponents.size !== 1) return "unavailable";
  const [component] = uniqueComponents;
  const packageName = component.slice(0, component.indexOf("/"));
  if (packageName === appId) return matchedState;
  if (packageName.startsWith(`${appId}.`) || appId.startsWith(`${packageName}.`)) {
    return "unavailable";
  }
  return otherState;
}

export function classifyAndroidActivityResumed(output, appId) {
  if (typeof output !== "string"
    || typeof appId !== "string"
    || !ANDROID_APP_ID_PATTERN.test(appId)) {
    return "unavailable";
  }
  const records = output.replaceAll("\r", "").split("\n").filter((line) => {
    const normalized = line.trim();
    return /^(?:mResumedActivity|topResumedActivity)\s*[:=]/.test(normalized)
      || /^ResumedActivity\s*:/.test(normalized);
  });
  if (records.length === 0) return "unavailable";
  return classifyExactPackageComponent(
    records,
    appId,
    /ActivityRecord\{([^{}]+)\}/g,
    "resumed",
    "not-resumed",
  );
}

export function classifyAndroidWindowFocused(output, appId) {
  if (typeof output !== "string"
    || typeof appId !== "string"
    || !ANDROID_APP_ID_PATTERN.test(appId)) {
    return "unavailable";
  }
  const records = output.replaceAll("\r", "").split("\n").filter((line) =>
    /^mCurrentFocus\s*[:=]/.test(line.trim()));
  if (records.length === 0) return "unavailable";
  return classifyExactPackageComponent(
    records,
    appId,
    /Window\{([^{}]+)\}/g,
    "focused",
    "not-focused",
  );
}

export function classifyAndroidNativeInputTargetObservation(activityOutput, windowOutput, appId) {
  const activityState = classifyAndroidActivityResumed(activityOutput, appId);
  const windowState = classifyAndroidWindowFocused(windowOutput, appId);
  const nativeTargetState = activityState === "unavailable" || windowState === "unavailable"
    ? "unavailable"
    : activityState === "resumed" && windowState === "focused" ? "ready" : "not-ready";
  return { activityState, windowState, nativeTargetState };
}

export function classifyAndroidNativeInputTarget(activityOutput, windowOutput, appId) {
  return classifyAndroidNativeInputTargetObservation(activityOutput, windowOutput, appId)
    .nativeTargetState;
}

async function waitForInputTarget(
  page,
  waitUntil,
  readNativeTargetState,
  timeout,
  label,
  onObservation = null,
) {
  let matchedSince = null;
  let matchedDocument = null;
  await waitUntil(label, async () => {
    let state;
    try {
      state = await page.call(() => ({
        documentTimeOrigin: performance.timeOrigin,
        visible: document.visibilityState === "visible",
        focused: document.hasFocus(),
        mobile: Boolean(document.querySelector('[data-deve-layout-mode="mobile"]')),
      }));
    } catch {
      matchedSince = null;
      matchedDocument = null;
      onObservation?.({
        documentIdentityValid: false,
        pageVisible: false,
        pageFocused: false,
        mobileLayout: false,
        nativeActivityState: "not-sampled",
        nativeWindowState: "not-sampled",
        nativeTargetState: "not-sampled",
      });
      throw new Error("android_webview_input_focus_sample_failed");
    }
    let nativeTargetState = "ready";
    let nativeActivityState = "not-sampled";
    let nativeWindowState = "not-sampled";
    let structuredNativeObservation = false;
    if (readNativeTargetState) {
      try {
        const nativeObservation = await readNativeTargetState();
        if (typeof nativeObservation === "string") {
          nativeTargetState = nativeObservation;
        } else {
          structuredNativeObservation = true;
          nativeTargetState = nativeObservation?.nativeTargetState;
          nativeActivityState = nativeObservation?.activityState;
          nativeWindowState = nativeObservation?.windowState;
        }
      } catch {
        matchedSince = null;
        matchedDocument = null;
        onObservation?.({
          documentIdentityValid: Number.isFinite(state?.documentTimeOrigin),
          pageVisible: state?.visible === true,
          pageFocused: state?.focused === true,
          mobileLayout: state?.mobile === true,
          nativeActivityState: "probe-failed",
          nativeWindowState: "probe-failed",
          nativeTargetState: "probe-failed",
        });
        throw new Error("android_native_input_target_sample_failed");
      }
    }
    let projectedNativeTargetState = ["ready", "not-ready", "unavailable"]
      .includes(nativeTargetState) ? nativeTargetState : "invalid";
    const projectedActivityState = ["resumed", "not-resumed", "unavailable", "not-sampled"]
      .includes(nativeActivityState) ? nativeActivityState : "invalid";
    const projectedWindowState = ["focused", "not-focused", "unavailable", "not-sampled"]
      .includes(nativeWindowState) ? nativeWindowState : "invalid";
    if (structuredNativeObservation) {
      const activitySampled = ["resumed", "not-resumed", "unavailable"]
        .includes(projectedActivityState);
      const windowSampled = ["focused", "not-focused", "unavailable"]
        .includes(projectedWindowState);
      if (!activitySampled || !windowSampled) {
        projectedNativeTargetState = "invalid";
      } else {
        const derivedNativeTargetState = projectedActivityState === "unavailable"
          || projectedWindowState === "unavailable"
          ? "unavailable"
          : projectedActivityState === "resumed" && projectedWindowState === "focused"
            ? "ready"
            : "not-ready";
        if (projectedNativeTargetState !== derivedNativeTargetState) {
          projectedNativeTargetState = "invalid";
        }
      }
    }
    onObservation?.({
      documentIdentityValid: Number.isFinite(state?.documentTimeOrigin),
      pageVisible: state?.visible === true,
      pageFocused: state?.focused === true,
      mobileLayout: state?.mobile === true,
      nativeActivityState: projectedActivityState,
      nativeWindowState: projectedWindowState,
      nativeTargetState: projectedNativeTargetState,
    });
    if (!Number.isFinite(state?.documentTimeOrigin)
      || !state.visible || !state.focused || !state.mobile
      || projectedNativeTargetState !== "ready") {
      matchedSince = null;
      matchedDocument = null;
      return false;
    }
    if (matchedDocument !== state.documentTimeOrigin) {
      matchedDocument = state.documentTimeOrigin;
      matchedSince = Date.now();
      return false;
    }
    return Date.now() - matchedSince >= WEBVIEW_INPUT_FOCUS_SETTLE_MS;
  }, timeout);
}

export async function waitForCurrentWebViewInputFocus(page, waitUntil, timeout = 30000) {
  await waitForInputTarget(
    page,
    waitUntil,
    null,
    timeout,
    "current Android WebView input focus settlement",
  );
}

export async function waitForCurrentAndroidWebViewInputTarget(
  page,
  waitUntil,
  readNativeTargetState,
  timeout = 30000,
) {
  if (typeof readNativeTargetState !== "function") {
    throw new Error("android_native_input_target_probe_missing");
  }
  let lastObservation = null;
  try {
    await waitForInputTarget(
      page,
      waitUntil,
      readNativeTargetState,
      timeout,
      "current Android native input target settlement",
      (observation) => { lastObservation = observation; },
    );
  } catch {
    throw new Error(
      `android_native_input_target_settlement_failed; last=${JSON.stringify(lastObservation)}`,
    );
  }
}

export function createAndroidWebViewInputTargetGate(
  adbOutput,
  appId,
  adbCommand,
  {
    allowForegroundReentry = false,
    settlementTimeoutMs = NATIVE_INPUT_REENTRY_TIMEOUT_MS,
    passiveTimeoutMs = NATIVE_INPUT_PASSIVE_TIMEOUT_MS,
    reentryTimeoutMs = NATIVE_INPUT_REENTRY_TIMEOUT_MS,
  } = {},
) {
  if (typeof adbOutput !== "function"
    || (allowForegroundReentry && typeof adbCommand !== "function")) {
    throw new Error("android_native_input_target_adb_probe_missing");
  }
  const strictTimeoutValid = Number.isFinite(settlementTimeoutMs) && settlementTimeoutMs > 0;
  const reentryTimeoutsValid = Number.isFinite(passiveTimeoutMs) && passiveTimeoutMs > 0
    && Number.isFinite(reentryTimeoutMs) && reentryTimeoutMs > 0;
  if (typeof appId !== "string" || !ANDROID_APP_ID_PATTERN.test(appId)
    || typeof allowForegroundReentry !== "boolean"
    || (allowForegroundReentry ? !reentryTimeoutsValid : !strictTimeoutValid)) {
    throw new Error("android_native_input_target_gate_config_invalid");
  }
  const readNativeTargetState = () => classifyAndroidNativeInputTargetObservation(
    adbOutput("shell", "dumpsys", "activity", "activities"),
    adbOutput("shell", "dumpsys", "window"),
    appId,
  );
  return async (page, waitUntil) => {
    if (!allowForegroundReentry) {
      await waitForCurrentAndroidWebViewInputTarget(
        page,
        waitUntil,
        readNativeTargetState,
        settlementTimeoutMs,
      );
      return;
    }
    try {
      await waitForCurrentAndroidWebViewInputTarget(
        page,
        waitUntil,
        readNativeTargetState,
        passiveTimeoutMs,
      );
      return;
    } catch {
      // A live WebView can temporarily lose the Android task/window input lease.
      // Reassert the existing launcher task once; business input is still sealed.
    }
    let pidBefore;
    try {
      pidBefore = adbOutput("shell", "pidof", appId).trim();
      if (!/^[1-9][0-9]*$/.test(pidBefore)) throw new Error("invalid pid");
      adbCommand(
        "shell", "monkey", "-p", appId,
        "-c", "android.intent.category.LAUNCHER", "1",
      );
    } catch {
      throw new Error("android_native_input_target_reentry_driver_failed");
    }
    try {
      await waitForCurrentAndroidWebViewInputTarget(
        page,
        waitUntil,
        readNativeTargetState,
        reentryTimeoutMs,
      );
    } catch (error) {
      throw new Error(`android_native_input_target_reentry_failed; ${error.message}`);
    }
    let pidAfter;
    try {
      pidAfter = adbOutput("shell", "pidof", appId).trim();
    } catch {
      throw new Error("android_native_input_target_reentry_pid_probe_failed");
    }
    if (pidAfter !== pidBefore) {
      throw new Error("android_native_input_target_reentry_pid_unstable");
    }
  };
}
