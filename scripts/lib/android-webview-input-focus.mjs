const WEBVIEW_INPUT_FOCUS_SETTLE_MS = 250;
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

export function classifyAndroidNativeInputTarget(activityOutput, windowOutput, appId) {
  const activityState = classifyAndroidActivityResumed(activityOutput, appId);
  const windowState = classifyAndroidWindowFocused(windowOutput, appId);
  if (activityState === "unavailable" || windowState === "unavailable") return "unavailable";
  return activityState === "resumed" && windowState === "focused" ? "ready" : "not-ready";
}

async function waitForInputTarget(page, waitUntil, readNativeTargetState, timeout, label) {
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
      throw new Error("android_webview_input_focus_sample_failed");
    }
    let nativeTargetState = "ready";
    if (readNativeTargetState) {
      try {
        nativeTargetState = await readNativeTargetState();
      } catch {
        matchedSince = null;
        matchedDocument = null;
        throw new Error("android_native_input_target_sample_failed");
      }
    }
    if (!Number.isFinite(state?.documentTimeOrigin)
      || !state.visible || !state.focused || !state.mobile
      || nativeTargetState !== "ready") {
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
  await waitForInputTarget(
    page,
    waitUntil,
    readNativeTargetState,
    timeout,
    "current Android native input target settlement",
  );
}

export function createAndroidWebViewInputTargetGate(adbOutput, appId) {
  if (typeof adbOutput !== "function") {
    throw new Error("android_native_input_target_adb_probe_missing");
  }
  return (page, waitUntil) => waitForCurrentAndroidWebViewInputTarget(
    page,
    waitUntil,
    () => classifyAndroidNativeInputTarget(
      adbOutput("shell", "dumpsys", "activity", "activities"),
      adbOutput("shell", "dumpsys", "window"),
      appId,
    ),
  );
}
