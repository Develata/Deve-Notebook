import { execFileSync } from "node:child_process";

export function classifyAndroidActivityResumed(output, appId) {
  if (typeof output !== "string"
    || typeof appId !== "string"
    || !/^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$/.test(appId)) {
    return "unavailable";
  }
  const records = output.replaceAll("\r", "").split("\n").filter((line) => {
    const normalized = line.trim();
    return /^(?:mResumedActivity|topResumedActivity)\s*[:=]/.test(normalized)
      || /^ResumedActivity\s*:/.test(normalized);
  });
  if (records.length === 0) return "unavailable";
  const components = records.map((line) => {
    const activityRecords = [...line.matchAll(/ActivityRecord\{([^{}]+)\}/g)];
    if (activityRecords.length !== 1) return null;
    const matches = [...activityRecords[0][1].matchAll(
      /(?:^|[\s{])([A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+)\/([A-Za-z0-9_.$]+)(?=[\s}])/g,
    )];
    return matches.length === 1 ? `${matches[0][1]}/${matches[0][2]}` : null;
  });
  if (components.some((component) => component === null)) return "unavailable";
  const uniqueComponents = new Set(components);
  if (uniqueComponents.size !== 1) return "unavailable";
  const [component] = uniqueComponents;
  const packageName = component.slice(0, component.indexOf("/"));
  if (packageName === appId) return "resumed";
  if (packageName.startsWith(`${appId}.`) || appId.startsWith(`${packageName}.`)) {
    return "unavailable";
  }
  return "not-resumed";
}

export function createAndroidLifecycleHarness({ timeoutMs, adb, serial }) {
  const harnessDeadline = Date.now() + timeoutMs;

  function remainingMs() {
    const remaining = harnessDeadline - Date.now();
    if (remaining <= 0) throw new Error("Android lifecycle harness deadline exhausted");
    return remaining;
  }

  function delay(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  async function withDeadline(label, promise, limit = remainingMs()) {
    let timer;
    try {
      return await Promise.race([
        promise,
        new Promise((_, reject) => {
          timer = setTimeout(
            () => reject(new Error(`timeout during ${label}`)),
            Math.min(limit, remainingMs()),
          );
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
  }

  async function waitUntil(label, predicate, timeout = Math.min(timeoutMs, 30000)) {
    const deadline = Math.min(Date.now() + timeout, harnessDeadline);
    let lastError;
    while (Date.now() < deadline) {
      try {
        const value = await withDeadline(
          `${label} predicate`,
          Promise.resolve().then(predicate),
          Math.max(1, deadline - Date.now()),
        );
        if (value) return value;
      } catch (error) {
        lastError = error;
      }
      await delay(Math.min(250, Math.max(1, deadline - Date.now())));
    }
    throw new Error(`timeout waiting for ${label}${lastError ? `: ${lastError.message}` : ""}`);
  }

  function adbCommand(...args) {
    execFileSync(adb, ["-s", serial, ...args], {
      stdio: "inherit",
      timeout: remainingMs(),
    });
  }

  function adbOutput(...args) {
    return execFileSync(adb, ["-s", serial, ...args], {
      encoding: "utf8",
      timeout: remainingMs(),
    }).replaceAll("\r", "");
  }

  async function tapAndroidEditor(point) {
    if (!Number.isFinite(point?.x)
      || !Number.isFinite(point?.y)
      || !Number.isFinite(point?.devicePixelRatio)
      || point.devicePixelRatio <= 0) {
      throw new Error("Android editor native tap point is invalid");
    }
    adbCommand(
      "shell", "input", "tap",
      String(Math.round(point.x * point.devicePixelRatio)),
      String(Math.round(point.y * point.devicePixelRatio)),
    );
    await delay(250);
  }

  async function proveAndroidRootBackBackground(appId, reentryReady) {
    const readPid = () => adbOutput("shell", "pidof", appId).trim();
    const resumedState = () => classifyAndroidActivityResumed(
      adbOutput("shell", "dumpsys", "activity", "activities"),
      appId,
    );
    const rootBackLogCount = () => adbOutput(
      "shell", "logcat", "-d", "-s", "DeveMobile:I", "*:S",
    ).split("\n").filter((line) => line.includes("android_ui_back_root_backgrounded")).length;

    const pidBefore = readPid();
    if (!pidBefore) throw new Error("Android root Back proof requires a live app PID");
    const logsBefore = rootBackLogCount();
    adbCommand("shell", "input", "keyevent", "4");
    await waitUntil("root Back backgrounds task", () => (
      resumedState() === "not-resumed"
        && readPid() === pidBefore
        && rootBackLogCount() > logsBefore
    ), 30000);
    adbCommand("shell", "monkey", "-p", appId, "-c", "android.intent.category.LAUNCHER", "1");
    await waitUntil("root Back task reentry", () => (
      resumedState() === "resumed" && readPid() === pidBefore
    ), 30000);
    await reentryReady();
    return { pidStable: readPid() === pidBefore, rootBackBackgrounded: true, reentryReady: true };
  }

  async function waitForAndroidRootReentry(readState, presentationBeforeRootBack) {
    return waitUntil("root Back lifecycle rebind", async () => {
      const { state, projection, presentation } = await readState();
      return state?.backend_running === true
        && state.service_state === "endpoint_session_ready"
        && projection.syncStatus === "handshaking-repo"
        && projection.repoIdRaw === ""
        && projection.loginVisible === false
        && projection.bootstrapSessionBound
        && projection.nativeSessionInstalled
        && presentation?.generation === presentationBeforeRootBack.generation
        && Number.isSafeInteger(presentation?.epoch)
        && presentation.epoch > presentationBeforeRootBack.epoch;
    }, 120000);
  }

  async function readAndroidUiBackSurfaceObservation(page) {
    return page.call(() => {
      const visible = (selector) => [...document.querySelectorAll(selector)].some((element) => {
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return style.display !== "none"
          && style.visibility !== "hidden"
          && rect.width > 0
          && rect.height > 0;
      });
      return {
        editorVisible: visible("[data-deve-editor-host=true]"),
        repoSwitcherVisible: visible("[data-deve-repo-switcher-menu]"),
        repoRemovalDialogVisible: visible('[data-deve-repo-removal-dialog="visible"]'),
        settingsVisible: visible('[data-deve-settings-surface="modal"]'),
        searchVisible: visible("[data-deve-search-sheet-position]"),
        mobileMoreMenuVisible: visible("[data-deve-mobile-more-menu]"),
        sourceControlMenuVisible: visible('[data-deve-sc-section-menu="true"]'),
        surfaceSwitcherVisible: visible('[data-deve-mobile-surface-sheet="open"]'),
        chatExpanded: visible('[data-deve-mobile-chat="expanded"]'),
        leftDrawerOpen: document.querySelector('[data-deve-mobile-drawer="left"]')
          ?.getAttribute("data-deve-mobile-drawer-open") === "true",
        rightDrawerOpen: document.querySelector('[data-deve-mobile-drawer="right"]')
          ?.getAttribute("data-deve-mobile-drawer-open") === "true",
        visibleDialogCount: [...document.querySelectorAll('[role="dialog"]')]
          .filter((element) => {
            const style = getComputedStyle(element);
            const rect = element.getBoundingClientRect();
            return style.display !== "none"
              && style.visibility !== "hidden"
              && rect.width > 0
              && rect.height > 0;
          }).length,
      };
    });
  }

  return {
    remainingMs,
    delay,
    withDeadline,
    waitUntil,
    adbCommand,
    adbOutput,
    tapAndroidEditor,
    proveAndroidRootBackBackground,
    waitForAndroidRootReentry,
    readAndroidUiBackSurfaceObservation,
  };
}
