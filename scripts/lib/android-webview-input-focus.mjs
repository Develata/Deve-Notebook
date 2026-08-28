import {
  classifyAndroidNativeInputTargetObservation,
} from "./android-native-input-observation.mjs";

export {
  classifyAndroidActivityResumed,
  classifyAndroidInputDispatcherFocused,
  classifyAndroidNativeInputTarget,
  classifyAndroidNativeInputTargetObservation,
  classifyAndroidWindowFocused,
} from "./android-native-input-observation.mjs";

const WEBVIEW_INPUT_FOCUS_SETTLE_MS = 250;
const NATIVE_INPUT_PASSIVE_TIMEOUT_MS = 1_500;
const NATIVE_INPUT_REENTRY_TIMEOUT_MS = 30_000;
const ANDROID_APP_ID_PATTERN = /^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$/;

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
        nativeInputDispatcherState: "not-sampled",
        nativeFocusState: "not-sampled",
        nativeTargetState: "not-sampled",
      });
      throw new Error("android_webview_input_focus_sample_failed");
    }
    let nativeTargetState = "ready";
    let nativeActivityState = "not-sampled";
    let nativeWindowState = "not-sampled";
    let nativeInputDispatcherState = "not-sampled";
    let nativeFocusState = "not-sampled";
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
          nativeInputDispatcherState = nativeObservation?.dispatcherState;
          nativeFocusState = nativeObservation?.focusState;
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
          nativeInputDispatcherState: "probe-failed",
          nativeFocusState: "probe-failed",
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
    const projectedInputDispatcherState = [
      "focused", "not-focused", "unavailable", "not-sampled",
    ].includes(nativeInputDispatcherState) ? nativeInputDispatcherState : "invalid";
    const projectedFocusState = ["focused", "not-focused", "unavailable", "not-sampled"]
      .includes(nativeFocusState) ? nativeFocusState : "invalid";
    if (structuredNativeObservation) {
      const activitySampled = ["resumed", "not-resumed", "unavailable"]
        .includes(projectedActivityState);
      const windowSampled = ["focused", "not-focused", "unavailable"]
        .includes(projectedWindowState);
      const dispatcherSampled = ["focused", "not-focused", "unavailable"]
        .includes(projectedInputDispatcherState);
      const focusSampled = ["focused", "not-focused", "unavailable"]
        .includes(projectedFocusState);
      if (!activitySampled || !windowSampled || !dispatcherSampled || !focusSampled) {
        projectedNativeTargetState = "invalid";
      } else {
        const derivedFocusState = projectedWindowState === "unavailable"
          ? projectedInputDispatcherState
          : projectedInputDispatcherState === "unavailable"
            ? projectedWindowState
            : projectedWindowState === projectedInputDispatcherState
              ? projectedWindowState
              : "unavailable";
        const derivedNativeTargetState = projectedActivityState === "unavailable"
          || derivedFocusState === "unavailable"
          ? "unavailable"
          : projectedActivityState === "resumed" && derivedFocusState === "focused"
            ? "ready"
            : "not-ready";
        if (projectedFocusState !== derivedFocusState
          || projectedNativeTargetState !== derivedNativeTargetState) {
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
      nativeInputDispatcherState: projectedInputDispatcherState,
      nativeFocusState: projectedFocusState,
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
  const readOptionalFocusProbe = (...args) => {
    try {
      return adbOutput(...args);
    } catch {
      return null;
    }
  };
  const readNativeTargetState = () => classifyAndroidNativeInputTargetObservation(
    adbOutput("shell", "dumpsys", "activity", "activities"),
    readOptionalFocusProbe("shell", "dumpsys", "window"),
    readOptionalFocusProbe("shell", "dumpsys", "input"),
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
